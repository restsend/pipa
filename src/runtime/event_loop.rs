use crate::runtime::context::JSContext;
use crate::runtime::extension::MacroTaskExtension;
use crate::value::JSValue;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub type TimerId = u32;

pub type AnimationCallbackId = u32;

pub struct Macrotask {
    pub callback: JSValue,

    pub args: Vec<JSValue>,

    pub timer_id: Option<TimerId>,

    pub is_interval: bool,

    pub interval: Option<Duration>,
}

#[derive(Clone)]
pub struct Timer {
    pub id: TimerId,

    pub callback: JSValue,

    pub args: Vec<JSValue>,

    pub deadline: Instant,

    pub is_interval: bool,

    pub interval: Option<Duration>,
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        other.deadline.cmp(&self.deadline)
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Timer {}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct AnimationCallback {
    pub id: AnimationCallbackId,

    pub callback: JSValue,
}

#[derive(Debug, Clone)]
pub struct EventLoopResult {
    pub completed: bool,

    pub macrotasks_executed: usize,

    pub timers_fired: usize,

    pub animation_callbacks_executed: usize,

    pub macrotasks_remaining: usize,

    pub timers_remaining: usize,

    pub animation_callbacks_remaining: usize,
}

pub struct EventLoop {
    macrotask_queue: VecDeque<Macrotask>,

    timer_heap: BinaryHeap<Timer>,

    active_timers: HashMap<TimerId, ()>,

    interval_timers: HashMap<TimerId, Timer>,

    animation_queue: VecDeque<AnimationCallback>,

    next_timer_id: TimerId,

    next_animation_id: AnimationCallbackId,

    max_iterations: u64,

    pub extensions: Vec<Box<dyn MacroTaskExtension>>,
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLoop {
    pub fn new() -> Self {
        EventLoop {
            macrotask_queue: VecDeque::new(),
            timer_heap: BinaryHeap::new(),
            active_timers: HashMap::new(),
            interval_timers: HashMap::new(),
            animation_queue: VecDeque::new(),
            next_timer_id: 1,
            next_animation_id: 1,
            max_iterations: 1_000_000,
            extensions: Vec::new(),
        }
    }

    pub fn set_max_iterations(&mut self, max: u64) {
        self.max_iterations = max;
    }

    pub fn schedule_timer(
        &mut self,
        callback: JSValue,
        args: Vec<JSValue>,
        delay_ms: u64,
        is_interval: bool,
    ) -> TimerId {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        let deadline = Instant::now() + Duration::from_millis(delay_ms);
        let interval = if is_interval {
            Some(Duration::from_millis(delay_ms))
        } else {
            None
        };

        let timer = Timer {
            id,
            callback,
            args,
            deadline,
            is_interval,
            interval,
        };

        self.timer_heap.push(timer.clone());
        self.active_timers.insert(id, ());

        if is_interval {
            self.interval_timers.insert(id, timer);
        }

        id
    }

    pub fn clear_timer(&mut self, timer_id: TimerId) {
        self.active_timers.remove(&timer_id);
        self.interval_timers.remove(&timer_id);
    }

    pub fn is_timer_active(&self, timer_id: TimerId) -> bool {
        self.active_timers.contains_key(&timer_id)
    }

    pub fn schedule_macrotask(&mut self, callback: JSValue, args: Vec<JSValue>) {
        self.macrotask_queue.push_back(Macrotask {
            callback,
            args,
            timer_id: None,
            is_interval: false,
            interval: None,
        });
    }

    pub fn schedule_animation_callback(&mut self, callback: JSValue) -> AnimationCallbackId {
        let id = self.next_animation_id;
        self.next_animation_id += 1;

        self.animation_queue
            .push_back(AnimationCallback { id, callback });

        id
    }

    pub fn cancel_animation_callback(&mut self, id: AnimationCallbackId) {
        self.animation_queue.retain(|cb| cb.id != id);
    }

    pub fn has_pending_tasks(&self) -> bool {
        !self.macrotask_queue.is_empty()
            || !self.timer_heap.is_empty()
            || !self.animation_queue.is_empty()
    }

    pub fn macrotask_count(&self) -> usize {
        self.macrotask_queue.len()
    }

    pub fn timer_count(&self) -> usize {
        self.timer_heap.len()
    }

    pub fn animation_callback_count(&self) -> usize {
        self.animation_queue.len()
    }

    fn fire_ready_timers(&mut self) -> usize {
        let mut fired = 0;
        let now = Instant::now();

        while let Some(timer) = self.timer_heap.pop() {
            if timer.deadline > now {
                self.timer_heap.push(timer);
                break;
            }

            if !self.active_timers.contains_key(&timer.id) {
                continue;
            }

            fired += 1;

            self.macrotask_queue.push_back(Macrotask {
                callback: timer.callback.clone(),
                args: timer.args.clone(),
                timer_id: Some(timer.id),
                is_interval: timer.is_interval,
                interval: timer.interval,
            });

            if !timer.is_interval {
                self.active_timers.remove(&timer.id);
            }
        }

        fired
    }

    fn execute_callback(
        &self,
        ctx: &mut JSContext,
        callback: JSValue,
        args: &[JSValue],
    ) -> Result<JSValue, String> {
        if !callback.is_function() {
            return Ok(JSValue::undefined());
        }

        let vm_ptr = ctx
            .get_register_vm_ptr()
            .ok_or("No VM associated with context")?;
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        vm.call_function(ctx, callback, args)
    }

    fn run_microtasks(&self, ctx: &mut JSContext) {
        let vm_ptr = match ctx.get_register_vm_ptr() {
            Some(ptr) => ptr,
            None => return,
        };
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        crate::builtins::promise::run_microtasks_with_vm(ctx, vm);
    }

    fn tick_extensions(&mut self, ctx: &mut JSContext) {
        for ext in &mut self.extensions {
            if let Err(e) = ext.tick(ctx) {
                eprintln!("[EventLoop] extension error: {e}");
            }
        }
    }

    pub fn run_until_complete(
        &mut self,
        ctx: &mut JSContext,
        timeout_ms: Option<u64>,
    ) -> Result<EventLoopResult, String> {
        let start = Instant::now();
        let timeout = timeout_ms.map(Duration::from_millis);
        let mut iterations = 0u64;

        let mut macrotasks_executed = 0usize;
        let mut timers_fired = 0usize;
        let mut animation_callbacks_executed = 0usize;

        loop {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(format!(
                    "Event loop iteration limit exceeded ({})",
                    self.max_iterations
                ));
            }

            if let Some(timeout_dur) = &timeout {
                if start.elapsed() > *timeout_dur {
                    return Ok(EventLoopResult {
                        completed: false,
                        macrotasks_executed,
                        timers_fired,
                        animation_callbacks_executed,
                        macrotasks_remaining: self.macrotask_queue.len(),
                        timers_remaining: self.timer_heap.len(),
                        animation_callbacks_remaining: self.animation_queue.len(),
                    });
                }
            }

            self.run_microtasks(ctx);

            self.tick_extensions(ctx);

            timers_fired += self.fire_ready_timers();

            let animation_callbacks: Vec<AnimationCallback> =
                self.animation_queue.drain(..).collect();
            for anim_cb in animation_callbacks {
                let timestamp = Instant::now().elapsed().as_nanos() as f64;
                let timestamp_val = JSValue::new_float(timestamp / 1_000_000.0);

                if anim_cb.callback.is_function() {
                    let _ = self.execute_callback(ctx, anim_cb.callback, &[timestamp_val]);
                    animation_callbacks_executed += 1;
                }
            }

            if let Some(macrotask) = self.macrotask_queue.pop_front() {
                let timer_id = macrotask.timer_id;
                let is_interval = macrotask.is_interval;
                let interval = macrotask.interval;

                if macrotask.callback.is_function() {
                    match self.execute_callback(ctx, macrotask.callback, &macrotask.args) {
                        Ok(_result) => {}
                        Err(e) => {
                            eprintln!("[EventLoop] Macrotask error: {}", e);
                        }
                    }
                    macrotasks_executed += 1;
                }

                if is_interval {
                    if let (Some(id), Some(interval_dur)) = (timer_id, interval) {
                        if self.active_timers.contains_key(&id) {
                            if let Some(stored_timer) = self.interval_timers.get(&id).cloned() {
                                let new_timer = Timer {
                                    id,
                                    callback: stored_timer.callback,
                                    args: stored_timer.args,
                                    deadline: Instant::now() + interval_dur,
                                    is_interval: true,
                                    interval: Some(interval_dur),
                                };
                                self.timer_heap.push(new_timer);
                            }
                        }
                    }
                }

                self.run_microtasks(ctx);
                continue;
            }

            let has_microtasks = !ctx.microtask_is_empty();
            let has_macrotasks = !self.macrotask_queue.is_empty();
            let has_timers = !self.timer_heap.is_empty();
            let has_animation = !self.animation_queue.is_empty();
            let has_io = self.extensions.iter().any(|e| e.has_pending());

            if !has_microtasks && !has_macrotasks && !has_timers && !has_animation && !has_io {
                return Ok(EventLoopResult {
                    completed: true,
                    macrotasks_executed,
                    timers_fired,
                    animation_callbacks_executed,
                    macrotasks_remaining: 0,
                    timers_remaining: 0,
                    animation_callbacks_remaining: 0,
                });
            }

            let exit_early = if has_microtasks || has_macrotasks || has_animation {
                false
            } else if has_timers {
                if let Some(next_timer) = self.timer_heap.peek() {
                    let wait = next_timer
                        .deadline
                        .saturating_duration_since(Instant::now());
                    wait > Duration::from_millis(100)
                } else {
                    true
                }
            } else if has_io {
                false
            } else {
                true
            };

            if exit_early {
                if has_io {
                    self.tick_extensions(ctx);
                    if !self.macrotask_queue.is_empty() {
                        continue;
                    }
                }
                return Ok(EventLoopResult {
                    completed: !has_io,
                    macrotasks_executed,
                    timers_fired,
                    animation_callbacks_executed,
                    macrotasks_remaining: self.macrotask_queue.len(),
                    timers_remaining: self.timer_heap.len(),
                    animation_callbacks_remaining: self.animation_queue.len(),
                });
            }
        }
    }

    pub fn tick(&mut self, ctx: &mut JSContext) -> Result<EventLoopResult, String> {
        let mut macrotasks_executed = 0usize;
        let mut timers_fired = 0usize;
        let mut animation_callbacks_executed = 0usize;

        self.run_microtasks(ctx);

        self.tick_extensions(ctx);

        timers_fired += self.fire_ready_timers();

        let animation_callbacks: Vec<AnimationCallback> = self.animation_queue.drain(..).collect();
        for anim_cb in animation_callbacks {
            let timestamp = Instant::now().elapsed().as_nanos() as f64;
            let timestamp_val = JSValue::new_float(timestamp / 1_000_000.0);

            if anim_cb.callback.is_function() {
                let _ = self.execute_callback(ctx, anim_cb.callback, &[timestamp_val]);
                animation_callbacks_executed += 1;
            }
        }

        if let Some(macrotask) = self.macrotask_queue.pop_front() {
            if macrotask.callback.is_function() {
                let _ = self.execute_callback(ctx, macrotask.callback, &macrotask.args);
                macrotasks_executed += 1;
            }

            self.run_microtasks(ctx);
        }

        Ok(EventLoopResult {
            completed: !self.has_pending_tasks()
                && ctx.microtask_is_empty()
                && !self.extensions.iter().any(|e| e.has_pending()),
            macrotasks_executed,
            timers_fired,
            animation_callbacks_executed,
            macrotasks_remaining: self.macrotask_queue.len(),
            timers_remaining: self.timer_heap.len(),
            animation_callbacks_remaining: self.animation_queue.len(),
        })
    }

    pub fn advance_time(
        &mut self,
        ctx: &mut JSContext,
        _duration_ms: u64,
    ) -> Result<EventLoopResult, String> {
        let mut timers_fired = 0usize;

        while let Some(timer) = self.timer_heap.pop() {
            if !self.active_timers.contains_key(&timer.id) {
                continue;
            }

            timers_fired += 1;

            self.macrotask_queue.push_back(Macrotask {
                callback: timer.callback.clone(),
                args: timer.args.clone(),
                timer_id: Some(timer.id),
                is_interval: timer.is_interval,
                interval: timer.interval,
            });

            if timer.is_interval {
                if let Some(interval) = timer.interval {
                    let mut new_timer = timer;
                    new_timer.deadline = Instant::now() + interval;
                    self.timer_heap.push(new_timer);
                }
            } else {
                self.active_timers.remove(&timer.id);
            }
        }

        let result = self.run_until_complete(ctx, None)?;

        Ok(EventLoopResult {
            timers_fired,
            ..result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_loop_creation() {
        let el = EventLoop::new();
        assert!(!el.has_pending_tasks());
        assert_eq!(el.macrotask_count(), 0);
        assert_eq!(el.timer_count(), 0);
    }

    #[test]
    fn test_schedule_macrotask() {
        let mut el = EventLoop::new();
        el.schedule_macrotask(JSValue::undefined(), vec![]);
        assert!(el.has_pending_tasks());
        assert_eq!(el.macrotask_count(), 1);
    }

    #[test]
    fn test_schedule_timer() {
        let mut el = EventLoop::new();
        let id = el.schedule_timer(JSValue::undefined(), vec![], 100, false);
        assert_eq!(id, 1);
        assert!(el.has_pending_tasks());
        assert_eq!(el.timer_count(), 1);
        assert!(el.is_timer_active(id));
    }

    #[test]
    fn test_clear_timer() {
        let mut el = EventLoop::new();
        let id = el.schedule_timer(JSValue::undefined(), vec![], 100, false);
        assert!(el.is_timer_active(id));
        el.clear_timer(id);
        assert!(!el.is_timer_active(id));
    }

    #[test]
    fn test_schedule_animation_callback() {
        let mut el = EventLoop::new();
        let id = el.schedule_animation_callback(JSValue::undefined());
        assert_eq!(id, 1);
        assert_eq!(el.animation_callback_count(), 1);
    }

    #[test]
    fn test_cancel_animation_callback() {
        let mut el = EventLoop::new();
        let id = el.schedule_animation_callback(JSValue::undefined());
        el.cancel_animation_callback(id);
        assert_eq!(el.animation_callback_count(), 0);
    }

    #[test]
    fn test_timer_ordering() {
        let mut el = EventLoop::new();

        let _id1 = el.schedule_timer(JSValue::undefined(), vec![], 300, false);
        let id2 = el.schedule_timer(JSValue::undefined(), vec![], 100, false);
        let _id3 = el.schedule_timer(JSValue::undefined(), vec![], 200, false);

        let top = el.timer_heap.peek().unwrap();
        assert_eq!(top.id, id2);
    }
}
