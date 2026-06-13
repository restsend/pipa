use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;
use std::collections::VecDeque;

const PROMISE_STATE_SLOT: &str = "__promise_state__";
const PROMISE_RESULT_SLOT: &str = "__promise_result__";
const PROMISE_REACTIONS_SLOT: &str = "__promise_reactions__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromiseState {
    Pending,
    Fulfilled,
    Rejected,
}

impl PromiseState {
    pub fn is_pending(&self) -> bool {
        matches!(self, PromiseState::Pending)
    }
}

#[derive(Clone)]
pub struct Reaction {
    pub handler: JSValue,
    pub is_reject: bool,
    pub is_all_settled: bool,
    pub is_finally: bool,

    pub target_promise: JSValue,
}

pub struct MicrotaskQueue {
    queue: VecDeque<Microtask>,
}

pub enum Microtask {
    Reaction(Reaction, JSValue),

    UserCallback(JSValue, Vec<JSValue>),
}

impl MicrotaskQueue {
    pub fn new() -> Self {
        MicrotaskQueue {
            queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, task: Microtask) {
        self.queue.push_back(task);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dequeue(&mut self) -> Option<Microtask> {
        self.queue.pop_front()
    }
}

impl Default for MicrotaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    if let Some(fp) = ctx.get_function_prototype() {
        func.base.prototype = Some(fp);
    }
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn intern_str(ctx: &mut JSContext, s: &str) -> crate::runtime::atom::Atom {
    ctx.intern(s)
}

fn get_array_element(obj: &JSObject, index: usize, ctx: &mut JSContext) -> JSValue {
    if obj.is_dense_array() {
        let ptr = obj as *const JSObject as *const crate::object::array_obj::JSArrayObject;
        let arr = unsafe { &*ptr };
        if index < arr.elements.len() {
            return arr.elements[index];
        }
        return JSValue::undefined();
    }
    if let Some(val) = obj.get_indexed(index) {
        return val;
    }
    let idx_atom = ctx.intern(&index.to_string());
    obj.get(idx_atom).unwrap_or(JSValue::undefined())
}

fn get_array_length(obj: &JSObject, ctx: &mut JSContext) -> usize {
    if obj.is_dense_array() {
        let ptr = obj as *const JSObject as *const crate::object::array_obj::JSArrayObject;
        let arr = unsafe { &*ptr };
        return arr.elements.len();
    }
    let length_atom = intern_str(ctx, "length");
    let len_val = obj.get(length_atom).unwrap_or(JSValue::new_int(0));
    if len_val.is_int() {
        len_val.get_int().max(0) as usize
    } else {
        0
    }
}

fn get_promise_state(ctx: &mut JSContext, obj: &JSObject) -> PromiseState {
    let state_atom = intern_str(ctx, PROMISE_STATE_SLOT);
    if let Some(state_val) = obj.get(state_atom) {
        let val = state_val.get_int();
        match val {
            1 => PromiseState::Fulfilled,
            2 => PromiseState::Rejected,
            _ => PromiseState::Pending,
        }
    } else {
        PromiseState::Pending
    }
}

fn set_promise_state(ctx: &mut JSContext, obj: &mut JSObject, state: PromiseState) {
    let state_val = match state {
        PromiseState::Pending => JSValue::new_int(0),
        PromiseState::Fulfilled => JSValue::new_int(1),
        PromiseState::Rejected => JSValue::new_int(2),
    };
    let state_atom = intern_str(ctx, PROMISE_STATE_SLOT);
    obj.set(state_atom, state_val);
}

fn get_promise_result(ctx: &mut JSContext, obj: &JSObject) -> JSValue {
    let result_atom = intern_str(ctx, PROMISE_RESULT_SLOT);
    obj.get(result_atom).unwrap_or(JSValue::undefined())
}

fn set_promise_result(ctx: &mut JSContext, obj: &mut JSObject, value: JSValue) {
    let result_atom = intern_str(ctx, PROMISE_RESULT_SLOT);
    obj.set(result_atom, value);
}

fn get_promise_reactions(ctx: &mut JSContext, obj: &JSObject) -> Vec<Reaction> {
    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
    if let Some(reactions_val) = obj.get(reactions_atom) {
        if reactions_val.is_object() {
            let ptr = reactions_val.get_ptr();

            if ptr == 0 {
                return Vec::new();
            }
            let arr = reactions_val.as_object();
            let len_atom = intern_str(ctx, "length");
            let len = arr.get(len_atom).map(|v| v.get_int() as usize).unwrap_or(0);
            let mut reactions = Vec::new();
            for i in 0..len {
                let idx_atom = intern_str(ctx, &i.to_string());
                if let Some(r_val) = arr.get(idx_atom) {
                    if r_val.is_object() {
                        let r_obj = r_val.as_object();
                        let handler_atom = intern_str(ctx, "handler");
                        let is_reject_atom = intern_str(ctx, "isReject");
                        let all_settled_atom = intern_str(ctx, "allSettled");
                        let finally_atom = intern_str(ctx, "isFinally");
                        let target_atom = intern_str(ctx, "target");
                        if let Some(handler) = r_obj.get(handler_atom) {
                            let is_reject = r_obj
                                .get(is_reject_atom)
                                .map(|v| v.get_bool())
                                .unwrap_or(false);
                            let is_all_settled = r_obj
                                .get(all_settled_atom)
                                .map(|v| v.get_bool())
                                .unwrap_or(false);
                            let is_finally = r_obj
                                .get(finally_atom)
                                .map(|v| v.get_bool())
                                .unwrap_or(false);
                            let target_promise = r_obj
                                .get(target_atom)
                                .unwrap_or(JSValue::undefined());
                            reactions.push(Reaction {
                                handler,
                                is_reject,
                                is_all_settled,
                                is_finally,
                                target_promise,
                            });
                        }
                    }
                }
            }
            return reactions;
        }
    }
    Vec::new()
}

fn create_reaction_array(ctx: &mut JSContext, reactions: Vec<Reaction>) -> JSValue {
    let mut arr = JSObject::new_array();
    let len_atom = intern_str(ctx, "length");
    arr.set(len_atom, JSValue::new_int(reactions.len() as i64));
    for (i, reaction) in reactions.into_iter().enumerate() {
        let mut r_obj = JSObject::new();
        let handler_atom = intern_str(ctx, "handler");
        let is_reject_atom = intern_str(ctx, "isReject");
        let all_settled_atom = intern_str(ctx, "allSettled");
        let finally_atom = intern_str(ctx, "isFinally");
        let target_atom = intern_str(ctx, "target");
        r_obj.set(handler_atom, reaction.handler);
        r_obj.set(is_reject_atom, JSValue::bool(reaction.is_reject));
        r_obj.set(all_settled_atom, JSValue::bool(reaction.is_all_settled));
        r_obj.set(finally_atom, JSValue::bool(reaction.is_finally));
        r_obj.set(target_atom, reaction.target_promise);
        let r_ptr = Box::into_raw(Box::new(r_obj)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(r_ptr);
        let idx_atom = intern_str(ctx, &i.to_string());
        arr.set(idx_atom, JSValue::new_object(r_ptr));
    }
    let ptr = Box::into_raw(Box::new(arr)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

pub fn is_promise(value: &JSValue) -> bool {
    if !value.is_object() {
        return false;
    }
    let obj = value.as_object();
    obj.is_promise()
}

fn create_promise(ctx: &mut JSContext) -> JSObject {
    let mut promise = JSObject::new_promise();

    if let Some(proto_ptr) = ctx.get_promise_prototype() {
        promise.prototype = Some(proto_ptr);
    }
    let state_atom = intern_str(ctx, PROMISE_STATE_SLOT);
    let result_atom = intern_str(ctx, PROMISE_RESULT_SLOT);
    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
    promise.set(state_atom, JSValue::new_int(0));
    promise.set(result_atom, JSValue::undefined());
    promise.set(reactions_atom, JSValue::null());
    promise
}

fn fulfill_promise(ctx: &mut JSContext, promise: &mut JSObject, value: JSValue) {
    set_promise_state(ctx, promise, PromiseState::Fulfilled);
    set_promise_result(ctx, promise, value);
    process_reactions(ctx, promise, PromiseState::Fulfilled);
}

fn reject_promise(ctx: &mut JSContext, promise: &mut JSObject, reason: JSValue) {
    set_promise_state(ctx, promise, PromiseState::Rejected);
    set_promise_result(ctx, promise, reason);
    process_reactions(ctx, promise, PromiseState::Rejected);
}

fn process_reactions(ctx: &mut JSContext, promise: &mut JSObject, state: PromiseState) {
    let reactions = get_promise_reactions(ctx, promise);
    for reaction in reactions {
        let argument = get_promise_result(ctx, promise);
        let microtask = if reaction.is_reject && state == PromiseState::Rejected {
            Microtask::Reaction(reaction, argument)
        } else if !reaction.is_reject && state == PromiseState::Fulfilled {
            Microtask::Reaction(reaction, argument)
        } else {
            continue;
        };
        ctx.microtask_enqueue(microtask);
    }
}

fn promise_resolve(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let value = if args.len() >= 2 {
        args[1]
    } else if !args.is_empty() {
        args[0]
    } else {
        JSValue::undefined()
    };
    if value.is_object() {
        let obj = value.as_object();
        if obj.is_promise() {
            return value;
        }
    }
    let mut promise = create_promise(ctx);
    fulfill_promise(ctx, &mut promise, value);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    JSValue::new_object(ptr)
}

fn promise_reject(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let reason = if args.len() >= 2 {
        args[1]
    } else if !args.is_empty() {
        args[0]
    } else {
        JSValue::undefined()
    };
    let mut promise = create_promise(ctx);
    reject_promise(ctx, &mut promise, reason);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    JSValue::new_object(ptr)
}

fn promise_then(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    if !this_val.is_object() {
        return JSValue::undefined();
    }
    let promise = this_val.as_object_mut();
    if !promise.is_promise() {
        return JSValue::undefined();
    }
    let on_fulfilled = args.get(1).copied().unwrap_or(JSValue::undefined());
    let on_rejected = args.get(2).copied().unwrap_or(JSValue::undefined());
    let new_promise = create_promise(ctx);
    let ptr_new = Box::into_raw(Box::new(new_promise)) as usize;
    let target = JSValue::new_object(ptr_new);
    let state = get_promise_state(ctx, promise);
    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
    match state {
        PromiseState::Fulfilled => {
            let result = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_fulfilled,
                is_reject: false,
                is_all_settled: false, is_finally: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, result));
        }
        PromiseState::Rejected => {
            let reason = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_rejected,
                is_reject: true,
                is_all_settled: false, is_finally: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, reason));
        }
        PromiseState::Pending => {
            let mut reactions = get_promise_reactions(ctx, promise);
            reactions.push(Reaction {
                handler: on_fulfilled,
                is_reject: false,
                is_all_settled: false, is_finally: false,
                target_promise: target,
            });
            reactions.push(Reaction {
                handler: on_rejected,
                is_reject: true,
                is_all_settled: false, is_finally: false,
                target_promise: target,
            });
            let reactions_arr = create_reaction_array(ctx, reactions);
            promise.set(reactions_atom, reactions_arr);
        }
    }
    target
}

fn promise_catch(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this_val = args.get(0).copied().unwrap_or(JSValue::undefined());
    let on_rejected = args.get(1).copied().unwrap_or(JSValue::undefined());

    let then_atom = ctx.intern("then");
    let then_fn = if this_val.is_object() {
        this_val.as_object().get(then_atom)
    } else {
        None
    };

    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        match then_fn {
            Some(f) if f.is_function() => {
                let result = vm.call_function_with_this(
                    ctx,
                    f,
                    this_val,
                    &[JSValue::undefined(), on_rejected],
                );
                match result {
                    Ok(v) => v,
                    Err(_) => {
                        if let Some(exc) = vm.last_caught_exception.take() {
                            vm.pending_throw = Some(exc);
                        }
                        JSValue::undefined()
                    }
                }
            }
            _ => {
                let mut err = JSObject::new();
                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Promise.prototype.catch called on non-Promise")));
                if let Some(proto) = ctx.get_type_error_prototype() {
                    err.prototype = Some(proto);
                }
                let ptr = Box::into_raw(Box::new(err)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(ptr);
                ctx.pending_exception = Some(JSValue::new_object(ptr));
                JSValue::undefined()
            }
        }
    } else {
        JSValue::undefined()
    }
}

fn promise_finally(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this_val = args.get(0).copied().unwrap_or(JSValue::undefined());
    let on_finally = args.get(1).copied().unwrap_or(JSValue::undefined());

    let then_atom = ctx.intern("then");
    let then_fn = if this_val.is_object() {
        this_val.as_object().get(then_atom)
    } else {
        None
    };

    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        match then_fn {
            Some(f) if f.is_function() => {
                let result = vm.call_function_with_this(
                    ctx,
                    f,
                    this_val,
                    &[on_finally, on_finally],
                );
                match result {
                    Ok(v) => v,
                    Err(_) => {
                        if let Some(exc) = vm.last_caught_exception.take() {
                            vm.pending_throw = Some(exc);
                        }
                        JSValue::undefined()
                    }
                }
            }
            _ => {
                let mut err = JSObject::new();
                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Promise.prototype.finally called on non-Promise")));
                if let Some(proto) = ctx.get_type_error_prototype() {
                    err.prototype = Some(proto);
                }
                let ptr = Box::into_raw(Box::new(err)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(ptr);
                ctx.pending_exception = Some(JSValue::new_object(ptr));
                JSValue::undefined()
            }
        }
    } else {
        JSValue::undefined()
    }
}

fn promise_all(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);
    ctx.runtime_mut().gc_heap_mut().track(result_promise_ptr);

    let rp = result_promise_val.as_object_mut();

    if !iterable.is_object() {
        let msg = JSValue::new_string(ctx.intern("TypeError: object is not iterable (cannot read property Symbol.iterator)"));
        reject_promise(ctx, rp, msg);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let sym_iter = crate::builtins::symbol::get_symbol_iterator(ctx);
    if sym_iter.is_symbol() {
        let sym_atom = crate::runtime::atom::Atom(0x40000000 | sym_iter.get_symbol_id());
        let iter_fn = obj.get(sym_atom).or_else(|| {
            let mut current = obj.prototype;
            while let Some(p) = current {
                let pobj = unsafe { &*p };
                if let Some(v) = pobj.get(sym_atom) {
                    return Some(v);
                }
                current = pobj.prototype;
            }
            None
        });
        match iter_fn {
            Some(f) if f.is_function() => {}
            _ => {
                let msg = JSValue::new_string(ctx.intern("TypeError: object is not iterable (cannot read property Symbol.iterator)"));
                reject_promise(ctx, rp, msg);
                return result_promise_val;
            }
        }
    }

    let len = get_array_length(&obj, ctx);

    if len == 0 {
        let mut empty_arr = JSObject::new_array();
        let length_atom = intern_str(ctx, "length");
        empty_arr.set(length_atom, JSValue::new_int(0));
        let arr_ptr = Box::into_raw(Box::new(empty_arr)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(arr_ptr);
        let rp = result_promise_val.as_object_mut();
        fulfill_promise(ctx, rp, JSValue::new_object(arr_ptr));
        return result_promise_val;
    }

    let mut results_arr = JSObject::new_array();
    let length_atom = intern_str(ctx, "length");
    results_arr.set(length_atom, JSValue::new_int(len as i64));
    let results_ptr = Box::into_raw(Box::new(results_arr)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(results_ptr);
    let results_val = JSValue::new_object(results_ptr);

    let remaining_atom = ctx.intern("__all_remaining__");
    let results_slot_atom = ctx.intern("__all_results__");
    let rp = result_promise_val.as_object_mut();
    rp.set(remaining_atom, JSValue::new_int(len as i64));
    rp.set(results_slot_atom, results_val);

    for i in 0..len {
        let item = get_array_element(&obj, i, ctx);

        let resolved = resolve_value_as_promise(ctx, item);
        let promise = resolved.as_object_mut();
        let state = get_promise_state(ctx, promise);
        match state {
            PromiseState::Fulfilled => {
                let value = get_promise_result(ctx, promise);
                let reaction = Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: false,
                    is_all_settled: false, is_finally: false,
                    target_promise: result_promise_val,
                };
                ctx.microtask_enqueue(Microtask::Reaction(reaction, value));
            }
            PromiseState::Rejected => {
                let reason = get_promise_result(ctx, promise);
                let rp = result_promise_val.as_object_mut();
                reject_promise(ctx, rp, reason);
                return result_promise_val;
            }
            PromiseState::Pending => {
                let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
                let mut reactions = get_promise_reactions(ctx, promise);
                reactions.push(Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: false,
                    is_all_settled: false, is_finally: false,
                    target_promise: result_promise_val,
                });
                reactions.push(Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: true,
                    is_all_settled: false, is_finally: false,
                    target_promise: result_promise_val,
                });
                let reactions_arr = create_reaction_array(ctx, reactions);
                promise.set(reactions_atom, reactions_arr);
            }
        }
    }

    result_promise_val
}

fn promise_race(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);
    ctx.runtime_mut().gc_heap_mut().track(result_promise_ptr);

    if !iterable.is_object() {
        let rp = result_promise_val.as_object_mut();
        let msg = JSValue::new_string(ctx.intern("TypeError: object is not iterable (cannot read property Symbol.iterator)"));
        reject_promise(ctx, rp, msg);
        return result_promise_val;
    }

    let obj = iterable.as_object();
    let len = get_array_length(&obj, ctx);

    for i in 0..len {
        let item = get_array_element(&obj, i, ctx);
        let resolved = resolve_value_as_promise(ctx, item);
        let promise = resolved.as_object_mut();
        let state = get_promise_state(ctx, promise);

        match state {
            PromiseState::Fulfilled => {
                let value = get_promise_result(ctx, promise);
                let rp = result_promise_val.as_object_mut();
                fulfill_promise(ctx, rp, value);
                return result_promise_val;
            }
            PromiseState::Rejected => {
                let reason = get_promise_result(ctx, promise);
                let rp = result_promise_val.as_object_mut();
                reject_promise(ctx, rp, reason);
                return result_promise_val;
            }
            PromiseState::Pending => {
                let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
                let mut reactions = get_promise_reactions(ctx, promise);
                reactions.push(Reaction {
                    handler: JSValue::new_int(-1),
                    is_reject: false,
                    is_all_settled: false, is_finally: false,
                    target_promise: result_promise_val,
                });
                reactions.push(Reaction {
                    handler: JSValue::new_int(-1),
                    is_reject: true,
                    is_all_settled: false, is_finally: false,
                    target_promise: result_promise_val,
                });
                let reactions_arr = create_reaction_array(ctx, reactions);
                promise.set(reactions_atom, reactions_arr);
            }
        }
    }

    result_promise_val
}

fn promise_all_settled(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);

    if !iterable.is_object() {
        let rp = result_promise_val.as_object_mut();
        let msg = JSValue::new_string(ctx.intern("TypeError: object is not iterable (cannot read property Symbol.iterator)"));
        reject_promise(ctx, rp, msg);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let len = get_array_length(&obj, ctx);

    if len == 0 {
        let mut empty_arr = JSObject::new_array();
        let length_atom = intern_str(ctx, "length");
        empty_arr.set(length_atom, JSValue::new_int(0));
        let arr_ptr = Box::into_raw(Box::new(empty_arr)) as usize;
        let rp = result_promise_val.as_object_mut();
        fulfill_promise(ctx, rp, JSValue::new_object(arr_ptr));
        ctx.runtime_mut().gc_heap_mut().track(arr_ptr);
        return result_promise_val;
    }

    let mut results_arr = JSObject::new_array();
    let length_atom = intern_str(ctx, "length");
    results_arr.set(length_atom, JSValue::new_int(len as i64));
    let results_ptr = Box::into_raw(Box::new(results_arr)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(results_ptr);
    let results_val = JSValue::new_object(results_ptr);

    let remaining_atom = ctx.intern("__allSettled_remaining__");
    let results_slot_atom = ctx.intern("__allSettled_results__");
    let rp = result_promise_val.as_object_mut();
    rp.set(remaining_atom, JSValue::new_int(len as i64));
    rp.set(results_slot_atom, results_val);

    for i in 0..len {
        let item = get_array_element(&obj, i, ctx);

        let resolved = resolve_value_as_promise(ctx, item);

        let promise = resolved.as_object_mut();
        let state = get_promise_state(ctx, promise);
        match state {
            PromiseState::Fulfilled => {
                let value = get_promise_result(ctx, promise);
                let reaction = Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: false,
                    is_all_settled: true, is_finally: false,
                    target_promise: result_promise_val,
                };
                ctx.microtask_enqueue(Microtask::Reaction(reaction, value));
            }
            PromiseState::Rejected => {
                let reason = get_promise_result(ctx, promise);
                let reaction = Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: true,
                    is_all_settled: true, is_finally: false,
                    target_promise: result_promise_val,
                };
                ctx.microtask_enqueue(Microtask::Reaction(reaction, reason));
            }
            PromiseState::Pending => {
                let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
                let mut reactions = get_promise_reactions(ctx, promise);
                reactions.push(Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: false,
                    is_all_settled: true, is_finally: false,
                    target_promise: result_promise_val,
                });
                reactions.push(Reaction {
                    handler: JSValue::new_int(i as i64),
                    is_reject: true,
                    is_all_settled: true, is_finally: false,
                    target_promise: result_promise_val,
                });
                let reactions_arr = create_reaction_array(ctx, reactions);
                promise.set(reactions_atom, reactions_arr);
            }
        }
    }

    result_promise_val
}

fn promise_any(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);

    if !iterable.is_object() {
        let msg = JSValue::new_string(ctx.intern("TypeError: object is not iterable (cannot read property Symbol.iterator)"));
        let rp = result_promise_val.as_object_mut();
        reject_promise(ctx, rp, msg);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let len = get_array_length(&obj, ctx);

    if len == 0 {
        let rp = result_promise_val.as_object_mut();
        let mut err = JSObject::new();
        err.set(intern_str(ctx, "name"), JSValue::new_string(ctx.intern("AggregateError")));
        err.set(intern_str(ctx, "message"), JSValue::new_string(ctx.intern("All promises were rejected")));
        if let Some(proto) = ctx.get_error_prototype() {
            err.prototype = Some(proto);
        }
        let err_ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(err_ptr);
        reject_promise(ctx, rp, JSValue::new_object(err_ptr));
        return result_promise_val;
    }

    let remaining_atom = ctx.intern("__any_remaining__");
    let rp = result_promise_val.as_object_mut();
    rp.set(remaining_atom, JSValue::new_int(len as i64));

    for i in 0..len {
        let item = get_array_element(&obj, i, ctx);

        let resolved = resolve_value_as_promise(ctx, item);
        let promise = resolved.as_object_mut();
        let state = get_promise_state(ctx, promise);

        match state {
            PromiseState::Fulfilled => {
                let value = get_promise_result(ctx, promise);
                let rp = result_promise_val.as_object_mut();
                fulfill_promise(ctx, rp, value);
                return result_promise_val;
            }
            PromiseState::Rejected | PromiseState::Pending => {
                if state == PromiseState::Pending {
                    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
                    let mut reactions = get_promise_reactions(ctx, promise);
                    reactions.push(Reaction {
                        handler: JSValue::new_int(i as i64),
                        is_reject: false,
                        is_all_settled: false, is_finally: false,
                        target_promise: result_promise_val,
                    });
                    reactions.push(Reaction {
                        handler: JSValue::new_int(i as i64),
                        is_reject: true,
                        is_all_settled: false, is_finally: false,
                        target_promise: result_promise_val,
                    });
                    let reactions_arr = create_reaction_array(ctx, reactions);
                    promise.set(reactions_atom, reactions_arr);
                } else {
                    let reason = get_promise_result(ctx, promise);
                    let reaction = Reaction {
                        handler: JSValue::new_int(i as i64),
                        is_reject: true,
                        is_all_settled: false, is_finally: false,
                        target_promise: result_promise_val,
                    };
                    ctx.microtask_enqueue(Microtask::Reaction(reaction, reason));
                }
            }
        }
    }

    result_promise_val
}

fn promise_with_resolvers(ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    let mut promise = create_promise(ctx);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    let promise_val = JSValue::new_object(ptr);

    let (resolve_val, reject_val) = create_resolve_reject_fns(ctx, promise_val);

    let mut result = JSObject::new();
    let promise_atom = ctx.intern("promise");
    let resolve_atom = ctx.intern("resolve");
    let reject_atom = ctx.intern("reject");
    result.set(promise_atom, promise_val);
    result.set(resolve_atom, resolve_val);
    result.set(reject_atom, reject_val);
    let result_ptr = Box::into_raw(Box::new(result)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(result_ptr);
    JSValue::new_object(result_ptr)
}

fn resolve_value_as_promise(ctx: &mut JSContext, value: JSValue) -> JSValue {
    if value.is_object() {
        let obj = value.as_object();
        if obj.is_promise() {
            return value;
        }
    }
    let mut promise = create_promise(ctx);
    fulfill_promise(ctx, &mut promise, value);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn promise_internal_resolve(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let callee_fn = args.get(0).copied().unwrap_or(JSValue::undefined());
    let value = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callee_fn.is_function() {
        return JSValue::undefined();
    }
    let target_atom = ctx.intern("__target_promise__");
    let func_obj = callee_fn.as_function();
    let promise_val = func_obj.base.get(target_atom).unwrap_or(JSValue::undefined());
    if !promise_val.is_object() {
        return JSValue::undefined();
    }
    let promise = promise_val.as_object_mut();
    if !promise.is_promise() || !get_promise_state(ctx, promise).is_pending() {
        return JSValue::undefined();
    }
    if value.is_object() {
        let val_obj = value.as_object();
        if val_obj.is_promise() {
            let state = get_promise_state(ctx, val_obj);
            let result_val = get_promise_result(ctx, val_obj);
            match state {
                PromiseState::Fulfilled => fulfill_promise(ctx, promise, result_val),
                PromiseState::Rejected => reject_promise(ctx, promise, result_val),
                PromiseState::Pending => {
                    let already_atom = ctx.intern("__already_resolved__");
                    promise.set(already_atom, JSValue::bool(true));
                    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
                    let mut reactions = get_promise_reactions(ctx, val_obj);
                    let target = promise_val;
                    reactions.push(Reaction {
                        handler: JSValue::undefined(),
                        is_reject: false,
                        is_all_settled: false, is_finally: false,
                        target_promise: target,
                    });
                    reactions.push(Reaction {
                        handler: JSValue::undefined(),
                        is_reject: true,
                        is_all_settled: false, is_finally: false,
                        target_promise: target,
                    });
                    let reactions_arr = create_reaction_array(ctx, reactions);
                    let val_mut = value.as_object_mut();
                    val_mut.set(reactions_atom, reactions_arr);
                    return JSValue::undefined();
                }
            }
            return JSValue::undefined();
        }
    }
    fulfill_promise(ctx, promise, value);
    JSValue::undefined()
}

fn promise_internal_reject(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let callee_fn = args.get(0).copied().unwrap_or(JSValue::undefined());
    let reason = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callee_fn.is_function() {
        return JSValue::undefined();
    }
    let target_atom = ctx.intern("__target_promise__");
    let func_obj = callee_fn.as_function();
    let promise_val = func_obj.base.get(target_atom).unwrap_or(JSValue::undefined());
    if !promise_val.is_object() {
        return JSValue::undefined();
    }
    let promise = promise_val.as_object_mut();
    if promise.is_promise() && get_promise_state(ctx, promise).is_pending() {
        reject_promise(ctx, promise, reason);
    }
    JSValue::undefined()
}

fn create_resolve_reject_fns(ctx: &mut JSContext, promise_val: JSValue) -> (JSValue, JSValue) {
    let empty_name = ctx.intern("");
    let mut resolve_fn = crate::object::function::JSFunction::new_builtin(empty_name, 1);
    resolve_fn.set_builtin_marker(ctx, "promise_internal_resolve");
    resolve_fn.name = empty_name;
    let name_desc = crate::object::object::PropertyDescriptor {
        value: Some(JSValue::new_string(empty_name)),
        writable: false,
        enumerable: false,
        configurable: true,
        get: None,
        set: None,
    };
    resolve_fn.base.define_property(ctx.common_atoms.name, name_desc);
    let resolve_target_atom = ctx.intern("__target_promise__");
    resolve_fn.base.set(resolve_target_atom, promise_val);
    let resolve_ptr = Box::into_raw(Box::new(resolve_fn)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(resolve_ptr);
    let resolve_val = JSValue::new_function(resolve_ptr);

    let mut reject_fn = crate::object::function::JSFunction::new_builtin(empty_name, 1);
    reject_fn.set_builtin_marker(ctx, "promise_internal_reject");
    reject_fn.name = empty_name;
    let name_desc = crate::object::object::PropertyDescriptor {
        value: Some(JSValue::new_string(empty_name)),
        writable: false,
        enumerable: false,
        configurable: true,
        get: None,
        set: None,
    };
    reject_fn.base.define_property(ctx.common_atoms.name, name_desc);
    let reject_target_atom = ctx.intern("__target_promise__");
    reject_fn.base.set(reject_target_atom, promise_val);
    let reject_ptr = Box::into_raw(Box::new(reject_fn)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(reject_ptr);
    let reject_val = JSValue::new_function(reject_ptr);

    (resolve_val, reject_val)
}

fn promise_executor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let executor_fn = args.get(0).copied().unwrap_or(JSValue::undefined());
    if !executor_fn.is_function() {
        let mut promise = create_promise(ctx);
        let ptr = Box::into_raw(Box::new(promise)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        let promise_val = JSValue::new_object(ptr);
        let rp = promise_val.as_object_mut();
        let msg = JSValue::new_string(ctx.intern("TypeError: Promise resolver undefined is not a function"));
        reject_promise(ctx, rp, msg);
        return promise_val;
    }

    let mut promise = create_promise(ctx);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    let promise_val = JSValue::new_object(ptr);

    let (resolve_val, reject_val) = create_resolve_reject_fns(ctx, promise_val);

    if let Err(e) = call_callback(ctx, executor_fn, &[resolve_val, reject_val]) {
        let rp = promise_val.as_object_mut();
        let msg = JSValue::new_string(ctx.intern(&e));
        reject_promise(ctx, rp, msg);
    }

    promise_val
}

fn call_callback(
    ctx: &mut JSContext,
    callback: JSValue,
    args: &[JSValue],
) -> Result<JSValue, String> {
    if let Some(ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
        vm.call_function(ctx, callback, args)
    } else {
        Err("VM not available".to_string())
    }
}

fn create_promise_prototype(ctx: &mut JSContext) -> JSValue {
    let mut proto = JSObject::new_promise();
    let constructor_atom = intern_str(ctx, "constructor");
    let then_atom = intern_str(ctx, "then");
    let catch_atom = intern_str(ctx, "catch");
    let finally_atom = intern_str(ctx, "finally");
    proto.set(constructor_atom, JSValue::null());

    use crate::builtins::global::set_non_enumerable;
    set_non_enumerable(&mut proto, then_atom, create_builtin_function(ctx, "promise_then"));
    set_non_enumerable(&mut proto, catch_atom, create_builtin_function(ctx, "promise_catch"));
    set_non_enumerable(&mut proto, finally_atom, create_builtin_function(ctx, "promise_finally"));

    let to_string_tag = crate::builtins::symbol::get_symbol_to_string_tag(ctx);
    if to_string_tag.is_symbol() {
        let sym_atom = crate::runtime::atom::Atom(0x40000000 | to_string_tag.get_symbol_id());
        set_non_enumerable(&mut proto, sym_atom, JSValue::new_string(ctx.intern("Promise")));
    }

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto.prototype = Some(obj_proto_ptr);
    }
    let ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

pub fn init_promise(ctx: &mut JSContext) {
    let promise_atom = ctx.intern("Promise");
    let mut promise_func = crate::object::function::JSFunction::new_builtin(promise_atom, 1);
    promise_func.set_builtin_marker(ctx, "promise_executor");
    let proto_value = create_promise_prototype(ctx);

    let proto_ptr = proto_value.get_ptr();
    ctx.set_promise_prototype(proto_ptr);
    use crate::builtins::global::set_non_enumerable;
    set_non_enumerable(&mut promise_func.base, ctx.intern("prototype"), proto_value);
    set_non_enumerable(&mut promise_func.base, ctx.intern("resolve"), create_builtin_function(ctx, "promise_resolve"));
    set_non_enumerable(&mut promise_func.base, ctx.intern("reject"), create_builtin_function(ctx, "promise_reject"));
    set_non_enumerable(&mut promise_func.base, ctx.intern("all"), create_builtin_function(ctx, "promise_all"));
    set_non_enumerable(&mut promise_func.base, ctx.intern("race"), create_builtin_function(ctx, "promise_race"));
    set_non_enumerable(&mut promise_func.base, ctx.intern("allSettled"), create_builtin_function(ctx, "promise_allSettled"));
    set_non_enumerable(&mut promise_func.base, ctx.intern("any"), create_builtin_function(ctx, "promise_any"));
    set_non_enumerable(&mut promise_func.base, ctx.intern("withResolvers"), create_builtin_function(ctx, "promise_with_resolvers"));
    let promise_ptr = Box::into_raw(Box::new(promise_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(promise_ptr);
    let promise_value = JSValue::new_function(promise_ptr);

    let proto_mut = unsafe { &mut *(proto_ptr as *mut JSObject) };
    proto_mut.set(ctx.intern("constructor"), promise_value);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, promise_atom, promise_value);
    }
}

pub fn create_resolved_promise(ctx: &mut JSContext, value: JSValue) -> JSValue {
    let mut promise = create_promise(ctx);
    fulfill_promise(ctx, &mut promise, value);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    let result = JSValue::new_object(ptr);
    result
}

pub fn create_rejected_promise(ctx: &mut JSContext, reason: JSValue) -> JSValue {
    let mut promise = create_promise(ctx);
    reject_promise(ctx, &mut promise, reason);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    JSValue::new_object(ptr)
}

#[cfg(any(feature = "process", feature = "fetch"))]
pub(crate) fn fulfill_promise_with_value(
    ctx: &mut JSContext,
    promise_obj_ptr: usize,
    value: JSValue,
) {
    let promise = unsafe { &mut *(promise_obj_ptr as *mut JSObject) };
    fulfill_promise(ctx, promise, value);
}

#[cfg(any(feature = "process", feature = "fetch"))]
pub(crate) fn reject_promise_with_value(
    ctx: &mut JSContext,
    promise_obj_ptr: usize,
    reason: JSValue,
) {
    let promise = unsafe { &mut *(promise_obj_ptr as *mut JSObject) };
    reject_promise(ctx, promise, reason);
}

#[cfg(feature = "fetch")]
pub fn create_pending_promise(ctx: &mut JSContext) -> JSValue {
    let promise = create_promise(ctx);
    let ptr = Box::into_raw(Box::new(promise)) as usize;
    JSValue::new_object(ptr)
}

pub fn run_microtasks_with_vm(ctx: &mut JSContext, vm: &mut crate::runtime::vm::VM) {
    while let Some(task) = ctx.microtask_dequeue() {
        match task {
            Microtask::Reaction(reaction, argument) => {
                if reaction.is_all_settled {
                    handle_all_settled_reaction(ctx, vm, &reaction, argument);
                    continue;
                }
                let handler = reaction.handler;
                if handler.is_int() && reaction.target_promise.is_object() {
                    let target = reaction.target_promise.as_object_mut();
                    if target.is_promise() && get_promise_state(ctx, target) == PromiseState::Pending {
                        let all_remaining_atom = ctx.intern("__all_remaining__");
                        let any_remaining_atom = ctx.intern("__any_remaining__");
                        if target.get(all_remaining_atom).is_some() {
                            if reaction.is_reject {
                                reject_promise(ctx, target, argument);
                            } else {
                                let idx = handler.get_int() as usize;
                                let results_slot = ctx.intern("__all_results__");
                                if let Some(results_val) = target.get(results_slot) {
                                    if results_val.is_object() {
                                        let results_arr = results_val.as_object_mut();
                                        let idx_atom = ctx.intern(&idx.to_string());
                                        results_arr.set(idx_atom, argument);
                                    }
                                }
                                let remaining = target.get(all_remaining_atom).unwrap().get_int() - 1;
                                target.set(all_remaining_atom, JSValue::new_int(remaining));
                                if remaining <= 0 {
                                    let results = target.get(results_slot).unwrap_or(JSValue::undefined());
                                    fulfill_promise(ctx, target, results);
                                }
                            }
                        } else if target.get(any_remaining_atom).is_some() {
                            if reaction.is_reject {
                                let remaining = target.get(any_remaining_atom).unwrap().get_int() - 1;
                                target.set(any_remaining_atom, JSValue::new_int(remaining));
                                if remaining <= 0 {
                                    let mut err = JSObject::new();
                                    err.set(intern_str(ctx, "name"), JSValue::new_string(ctx.intern("AggregateError")));
                                    err.set(intern_str(ctx, "message"), JSValue::new_string(ctx.intern("All promises were rejected")));
                                    if let Some(proto) = ctx.get_error_prototype() {
                                        err.prototype = Some(proto);
                                    }
                                    let err_ptr = Box::into_raw(Box::new(err)) as usize;
                                    ctx.runtime_mut().gc_heap_mut().track(err_ptr);
                                    reject_promise(ctx, target, JSValue::new_object(err_ptr));
                                }
                            } else {
                                fulfill_promise(ctx, target, argument);
                            }
                        } else {
                            if reaction.is_reject {
                                reject_promise(ctx, target, argument);
                            } else {
                                fulfill_promise(ctx, target, argument);
                            }
                        }
                    }
                    continue;
                }
                if handler.is_function() {
                    let result = vm.call_function(ctx, handler, &[argument]);
                    if reaction.target_promise.is_object() {
                        let target = reaction.target_promise.as_object_mut();
                        if target.is_promise() {
                            if reaction.is_finally {
                                match result {
                                    Ok(_) => {
                                        if reaction.is_reject {
                                            reject_promise(ctx, target, argument);
                                        } else {
                                            fulfill_promise(ctx, target, argument);
                                        }
                                    }
                                    Err(e) => {
                                        let msg = JSValue::new_string(ctx.intern(&e));
                                        reject_promise(ctx, target, msg);
                                    }
                                }
                            } else {
                                match result {
                                    Ok(val) => fulfill_promise(ctx, target, val),
                                    Err(_) => {
                                        reject_promise(ctx, target, JSValue::undefined());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Microtask::UserCallback(callback, args) => {
                if callback.is_function() {
                    let _result = vm.call_function(ctx, callback, &args);
                }
            }
        }
    }
}

fn handle_all_settled_reaction(ctx: &mut JSContext, vm: &mut crate::runtime::vm::VM, reaction: &Reaction, argument: JSValue) {
    let idx = reaction.handler.get_int() as usize;
    let target = reaction.target_promise;
    if !target.is_object() {
        return;
    }

    let mut result_obj = JSObject::new();
    let status_atom = ctx.intern("status");
    if reaction.is_reject {
        result_obj.set(status_atom, JSValue::new_string(ctx.intern("rejected")));
        result_obj.set(ctx.intern("reason"), argument);
    } else {
        result_obj.set(status_atom, JSValue::new_string(ctx.intern("fulfilled")));
        result_obj.set(ctx.intern("value"), argument);
    }
    let r_ptr = Box::into_raw(Box::new(result_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(r_ptr);
    let result_val = JSValue::new_object(r_ptr);

    let target_obj = target.as_object_mut();
    let results_slot = ctx.intern("__allSettled_results__");
    if let Some(results_val) = target_obj.get(results_slot) {
        if results_val.is_object() {
            let results_arr = results_val.as_object_mut();
            let idx_atom = ctx.intern(&idx.to_string());
            results_arr.set(idx_atom, result_val);
        }
    }

    let remaining_atom = ctx.intern("__allSettled_remaining__");
    if let Some(rem_val) = target_obj.get(remaining_atom) {
        let remaining = rem_val.get_int() - 1;
        target_obj.set(remaining_atom, JSValue::new_int(remaining));
        if remaining <= 0 {
            let results = target_obj.get(results_slot).unwrap_or(JSValue::undefined());
            fulfill_promise(ctx, target_obj, results);
        }
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "promise_executor",
        HostFunction::ctor("executor", 1, promise_executor),
    );
    ctx.register_builtin(
        "promise_internal_resolve",
        HostFunction::new("resolve", 1, promise_internal_resolve),
    );
    ctx.register_builtin(
        "promise_internal_reject",
        HostFunction::new("reject", 1, promise_internal_reject),
    );
    ctx.register_builtin(
        "promise_resolve",
        HostFunction::new("resolve", 1, promise_resolve),
    );
    ctx.register_builtin(
        "promise_reject",
        HostFunction::new("reject", 1, promise_reject),
    );
    ctx.register_builtin(
        "promise_then",
        HostFunction::method("then", 2, promise_then),
    );
    ctx.register_builtin(
        "promise_catch",
        HostFunction::method("catch", 2, promise_catch),
    );
    ctx.register_builtin(
        "promise_finally",
        HostFunction::method("finally", 2, promise_finally),
    );
    ctx.register_builtin("promise_all", HostFunction::new("all", 1, promise_all));
    ctx.register_builtin("promise_race", HostFunction::new("race", 1, promise_race));
    ctx.register_builtin(
        "promise_allSettled",
        HostFunction::new("allSettled", 1, promise_all_settled),
    );
    ctx.register_builtin("promise_any", HostFunction::new("any", 1, promise_any));
    ctx.register_builtin(
        "promise_with_resolvers",
        HostFunction::new("withResolvers", 0, promise_with_resolvers),
    );
}
