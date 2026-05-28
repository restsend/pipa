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

#[derive(Clone)]
pub struct Reaction {
    pub handler: JSValue,
    pub is_reject: bool,
    pub is_all_settled: bool,

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
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn intern_str(ctx: &mut JSContext, s: &str) -> crate::runtime::atom::Atom {
    ctx.intern(s)
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
                        if let Some(handler) = r_obj.get(handler_atom) {
                            let is_reject = r_obj
                                .get(is_reject_atom)
                                .map(|v| v.get_bool())
                                .unwrap_or(false);
                            reactions.push(Reaction {
                                handler,
                                is_reject,
                                is_all_settled: false,
                                target_promise: JSValue::undefined(),
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
        r_obj.set(handler_atom, reaction.handler);
        r_obj.set(is_reject_atom, JSValue::bool(reaction.is_reject));
        let r_ptr = Box::into_raw(Box::new(r_obj)) as usize;
        let idx_atom = intern_str(ctx, &i.to_string());
        arr.set(idx_atom, JSValue::new_object(r_ptr));
    }
    let ptr = Box::into_raw(Box::new(arr)) as usize;
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
                is_all_settled: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, result));
        }
        PromiseState::Rejected => {
            let reason = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_rejected,
                is_reject: true,
                is_all_settled: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, reason));
        }
        PromiseState::Pending => {
            let mut reactions = get_promise_reactions(ctx, promise);
            reactions.push(Reaction {
                handler: on_fulfilled,
                is_reject: false,
                is_all_settled: false,
                target_promise: target,
            });
            reactions.push(Reaction {
                handler: on_rejected,
                is_reject: true,
                is_all_settled: false,
                target_promise: target,
            });
            let reactions_arr = create_reaction_array(ctx, reactions);
            promise.set(reactions_atom, reactions_arr);
        }
    }
    target
}

fn promise_catch(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    let on_rejected = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !this_val.is_object() {
        return JSValue::undefined();
    }
    let promise = this_val.as_object_mut();
    if !promise.is_promise() {
        return JSValue::undefined();
    }
    let new_promise = create_promise(ctx);
    let ptr_new = Box::into_raw(Box::new(new_promise)) as usize;
    let target = JSValue::new_object(ptr_new);
    let state = get_promise_state(ctx, promise);
    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
    match state {
        PromiseState::Fulfilled => {
            let result = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_rejected,
                is_reject: false,
                is_all_settled: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, result));
        }
        PromiseState::Rejected => {
            let reason = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_rejected,
                is_reject: true,
                is_all_settled: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, reason));
        }
        PromiseState::Pending => {
            let mut reactions = get_promise_reactions(ctx, promise);
            reactions.push(Reaction {
                handler: on_rejected,
                is_reject: true,
                is_all_settled: false,
                target_promise: target,
            });
            let reactions_arr = create_reaction_array(ctx, reactions);
            promise.set(reactions_atom, reactions_arr);
        }
    }
    target
}

fn promise_finally(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    let on_finally = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !this_val.is_object() {
        return JSValue::undefined();
    }
    let promise = this_val.as_object_mut();
    if !promise.is_promise() {
        return JSValue::undefined();
    }
    let new_promise = create_promise(ctx);
    let ptr_new = Box::into_raw(Box::new(new_promise)) as usize;
    let target = JSValue::new_object(ptr_new);
    let state = get_promise_state(ctx, promise);
    let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
    match state {
        PromiseState::Fulfilled => {
            let result = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_finally,
                is_reject: false,
                is_all_settled: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, result));
        }
        PromiseState::Rejected => {
            let reason = get_promise_result(ctx, promise);
            let reaction = Reaction {
                handler: on_finally,
                is_reject: true,
                is_all_settled: false,
                target_promise: target,
            };
            ctx.microtask_enqueue(Microtask::Reaction(reaction, reason));
        }
        PromiseState::Pending => {
            let mut reactions = get_promise_reactions(ctx, promise);
            reactions.push(Reaction {
                handler: on_finally,
                is_reject: false,
                is_all_settled: false,
                target_promise: target,
            });
            reactions.push(Reaction {
                handler: on_finally,
                is_reject: true,
                is_all_settled: false,
                target_promise: target,
            });
            let reactions_arr = create_reaction_array(ctx, reactions);
            promise.set(reactions_atom, reactions_arr);
        }
    }
    target
}

fn promise_all(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);

    if !iterable.is_object() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.all requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let length_atom = intern_str(ctx, "length");
    let len_val = obj.get(length_atom).unwrap_or(JSValue::undefined());
    if !len_val.is_int() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.all requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let len = len_val.get_int() as usize;

    let pending_atom = ctx.intern("__promise_all_pending__");
    let result_promise_mut = result_promise_val.as_object_mut();
    result_promise_mut.set(pending_atom, JSValue::new_int(len as i64));

    for i in 0..len {
        let idx_atom = ctx.intern(&i.to_string());
        let item = obj.get(idx_atom).unwrap_or(JSValue::undefined());

        let reaction = Reaction {
            handler: JSValue::new_int(i as i64),
            is_reject: false,
            is_all_settled: false,
            target_promise: JSValue::undefined(),
        };
        ctx.microtask_enqueue(Microtask::Reaction(reaction, item));
    }

    result_promise_val
}

fn promise_race(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);

    if !iterable.is_object() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.race requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let length_atom = intern_str(ctx, "length");
    let len_val = obj.get(length_atom).unwrap_or(JSValue::undefined());
    if !len_val.is_int() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.race requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let len = len_val.get_int() as usize;

    if len == 0 {
        return result_promise_val;
    }

    for i in 0..len {
        let idx_atom = ctx.intern(&i.to_string());
        let item = obj.get(idx_atom).unwrap_or(JSValue::undefined());

        let reaction = Reaction {
            handler: JSValue::new_int(i as i64),
            is_reject: false,
            is_all_settled: false,
            target_promise: JSValue::undefined(),
        };
        ctx.microtask_enqueue(Microtask::Reaction(reaction, item));
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
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.allSettled requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let length_atom = intern_str(ctx, "length");
    let len_val = obj.get(length_atom).unwrap_or(JSValue::undefined());
    if !len_val.is_int() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.allSettled requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let len = len_val.get_int() as usize;

    let pending_atom = intern_str(ctx, "__promise_all_pending__");
    let result_promise_mut = result_promise_val.as_object_mut();
    result_promise_mut.set(pending_atom, JSValue::new_int(len as i64));

    for i in 0..len {
        let idx_atom = ctx.intern(&i.to_string());
        let item = obj.get(idx_atom).unwrap_or(JSValue::undefined());

        let reaction = Reaction {
            handler: JSValue::new_int(i as i64),
            is_reject: false,
            is_all_settled: true,
            target_promise: JSValue::undefined(),
        };
        ctx.microtask_enqueue(Microtask::Reaction(reaction, item));
    }

    result_promise_val
}

fn promise_any(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let iterable = args.get(0).copied().unwrap_or(JSValue::undefined());

    let result_promise = create_promise(ctx);
    let result_promise_ptr = Box::into_raw(Box::new(result_promise)) as usize;
    let result_promise_val = JSValue::new_object(result_promise_ptr);

    if !iterable.is_object() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.any requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let obj = iterable.as_object();

    let length_atom = intern_str(ctx, "length");
    let len_val = obj.get(length_atom).unwrap_or(JSValue::undefined());
    if !len_val.is_int() {
        let rp = result_promise_val.as_object_mut();
        let err_val =
            JSValue::new_string(ctx.intern("TypeError: Promise.any requires an iterable"));
        set_promise_result(ctx, rp, err_val);
        set_promise_state(ctx, rp, PromiseState::Rejected);
        return result_promise_val;
    }

    let len = len_val.get_int() as usize;

    let pending_atom = intern_str(ctx, "__promise_any_pending__");
    let result_promise_mut = result_promise_val.as_object_mut();
    result_promise_mut.set(pending_atom, JSValue::new_int(len as i64));

    for i in 0..len {
        let idx_atom = ctx.intern(&i.to_string());
        let item = obj.get(idx_atom).unwrap_or(JSValue::undefined());

        let reaction = Reaction {
            handler: JSValue::new_int(i as i64),
            is_reject: false,
            is_all_settled: false,
            target_promise: JSValue::undefined(),
        };
        ctx.microtask_enqueue(Microtask::Reaction(reaction, item));
    }

    result_promise_val
}

fn promise_executor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 {
        return JSValue::undefined();
    }
    let promise_val = args[0];
    let resolve_fn = args[1];
    let reject_fn = args[2];
    if !promise_val.is_object() || !resolve_fn.is_function() || !reject_fn.is_function() {
        return JSValue::undefined();
    }
    let promise_obj = promise_val.as_object_mut();
    let result = get_promise_result(ctx, promise_obj);
    if let Ok(result) = call_callback(ctx, resolve_fn, &[result]) {
        if result.is_object() {
            let result_obj = result.as_object_mut();
            if result_obj.is_promise() {
                let state = get_promise_state(ctx, result_obj);
                let result_val = get_promise_result(ctx, result_obj);
                let reactions_atom = intern_str(ctx, PROMISE_REACTIONS_SLOT);
                match state {
                    PromiseState::Fulfilled => fulfill_promise(ctx, promise_obj, result_val),
                    PromiseState::Rejected => reject_promise(ctx, promise_obj, result_val),
                    PromiseState::Pending => {
                        let mut reactions = get_promise_reactions(ctx, result_obj);
                        reactions.push(Reaction {
                            handler: promise_val,
                            is_reject: false,
                            is_all_settled: false,
                            target_promise: JSValue::undefined(),
                        });
                        let reactions_arr = create_reaction_array(ctx, reactions);
                        result_obj.set(reactions_atom, reactions_arr);
                    }
                }
            } else {
                fulfill_promise(ctx, promise_obj, result);
            }
        } else {
            fulfill_promise(ctx, promise_obj, result);
        }
    }
    JSValue::undefined()
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
    proto.set(then_atom, create_builtin_function(ctx, "promise_then"));
    proto.set(catch_atom, create_builtin_function(ctx, "promise_catch"));
    proto.set(
        finally_atom,
        create_builtin_function(ctx, "promise_finally"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto.prototype = Some(obj_proto_ptr);
    }
    let ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

pub fn init_promise(ctx: &mut JSContext) {
    let promise_atom = ctx.intern("Promise");
    let mut promise_obj = JSObject::new_function();
    let proto_value = create_promise_prototype(ctx);

    let proto_ptr = proto_value.get_ptr();
    ctx.set_promise_prototype(proto_ptr);
    promise_obj.set(ctx.intern("prototype"), proto_value);
    promise_obj.set(
        ctx.intern("resolve"),
        create_builtin_function(ctx, "promise_resolve"),
    );
    promise_obj.set(
        ctx.intern("reject"),
        create_builtin_function(ctx, "promise_reject"),
    );
    promise_obj.set(
        ctx.intern("all"),
        create_builtin_function(ctx, "promise_all"),
    );
    promise_obj.set(
        ctx.intern("race"),
        create_builtin_function(ctx, "promise_race"),
    );
    promise_obj.set(
        ctx.intern("allSettled"),
        create_builtin_function(ctx, "promise_allSettled"),
    );
    promise_obj.set(
        ctx.intern("any"),
        create_builtin_function(ctx, "promise_any"),
    );
    let promise_ptr = Box::into_raw(Box::new(promise_obj)) as usize;
    let promise_value = JSValue::new_object(promise_ptr);
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
                let handler = reaction.handler;
                if handler.is_function() {
                    let result = vm.call_function(ctx, handler, &[argument]);
                    if reaction.target_promise.is_object() {
                        let target = reaction.target_promise.as_object_mut();
                        if target.is_promise() {
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
            Microtask::UserCallback(callback, args) => {
                if callback.is_function() {
                    let _result = vm.call_function(ctx, callback, &args);
                }
            }
        }
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "promise_executor",
        HostFunction::ctor("executor", 3, promise_executor),
    );
    ctx.register_builtin(
        "promise_resolve",
        HostFunction::new("resolve", 1, promise_resolve),
    );
    ctx.register_builtin(
        "promise_reject",
        HostFunction::new("reject", 1, promise_reject),
    );
    ctx.register_builtin("promise_then", HostFunction::method("then", 2, promise_then));
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
}
