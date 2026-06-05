use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn register_builtins(_ctx: &mut JSContext) {}

pub fn init_generator(ctx: &mut JSContext) {
    let mut gen_proto = JSObject::new();

    ctx.register_builtin(
        "generator_next",
        HostFunction::method("next", 1, generator_next),
    );

    let next_func = create_builtin_function(ctx, "generator_next");

    let next_atom = ctx.intern("next");
    gen_proto.set(next_atom, next_func);

    ctx.register_builtin(
        "generator_return",
        HostFunction::new("return", 1, generator_return),
    );
    let return_func = create_builtin_function(ctx, "generator_return");
    let return_atom = ctx.intern("return");
    gen_proto.set(return_atom, return_func);

    // Generator.prototype[Symbol.iterator] = function() { return this; }
    ctx.register_builtin(
        "generator_symbol_iterator",
        HostFunction::method("[Symbol.iterator]", 0, generator_symbol_iterator),
    );
    let sym_iter_func = create_builtin_function(ctx, "generator_symbol_iterator");
    let sym_iter_val = crate::builtins::symbol::get_symbol_iterator(ctx);
    if sym_iter_val.is_symbol() {
        let sym_atom = crate::runtime::atom::Atom(0x40000000 | sym_iter_val.get_symbol_id());
        gen_proto.set(sym_atom, sym_iter_func);
    }

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        gen_proto.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(gen_proto)) as usize;
    ctx.set_generator_prototype(proto_ptr);

    let mut async_gen_proto = JSObject::new();

    ctx.register_builtin(
        "async_generator_next",
        HostFunction::new("next", 1, async_generator_next),
    );
    let async_next_func = create_builtin_function(ctx, "async_generator_next");
    async_gen_proto.set(next_atom, async_next_func);

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        async_gen_proto.prototype = Some(obj_proto_ptr);
    }

    let async_proto_ptr = Box::into_raw(Box::new(async_gen_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(async_proto_ptr);
    ctx.set_async_generator_prototype(async_proto_ptr);
}

fn create_builtin_function(ctx: &mut JSContext, builtin_name: &str) -> JSValue {
    let name_atom = ctx.intern(builtin_name);
    let mut func = JSFunction::new_builtin(name_atom, 1);
    func.set_builtin_marker(ctx, builtin_name);
    let proto_atom = ctx.intern("prototype");
    let proto_obj = JSObject::new();
    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    func.base.set(proto_atom, JSValue::new_object(proto_ptr));

    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn run_generator_step(
    ctx: &mut JSContext,
    gen_obj: &mut JSObject,
    _send_value: JSValue,
) -> Result<(JSValue, usize, bool), String> {
    let state = match gen_obj.get_generator_state() {
        Some(s) => s,
        None => return Ok((JSValue::undefined(), 0, true)),
    };

    if state.done {
        return Ok((JSValue::undefined(), state.pc, true));
    }

    let current_pc = state.pc;
    let snapshot_clone = state.snapshot.clone();

    let (value, new_snapshot, new_pc, done) = {
        let bytecode = &*gen_obj.get_generator_state().unwrap().bytecode;
        let mut vm = crate::runtime::vm::VM::new();

        ctx.runtime_mut().gc_heap_mut().gc_suppressed = true;
        let result = vm.execute_generator_step(ctx, bytecode, &snapshot_clone, current_pc);
        ctx.runtime_mut().gc_heap_mut().gc_suppressed = false;
        result?
    };

    if let Some(state) = gen_obj.get_generator_state_mut() {
        state.snapshot = new_snapshot;
        state.pc = new_pc;
        state.done = done;
    }

    Ok((value, new_pc, done))
}

fn generator_next(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = args.get(0).copied().unwrap_or(JSValue::undefined());

    if !this.is_object() {
        return create_iterator_result(ctx, JSValue::undefined(), true);
    }

    let gen_obj = this.as_object_mut();

    if !gen_obj.is_generator() {
        return create_iterator_result(ctx, JSValue::undefined(), true);
    }

    let is_done = gen_obj
        .get_generator_state()
        .map(|s| s.done)
        .unwrap_or(true);

    if is_done {
        return create_iterator_result(ctx, JSValue::undefined(), true);
    }

    let send_value = args.get(1).copied().unwrap_or(JSValue::undefined());

    match run_generator_step(ctx, gen_obj, send_value) {
        Ok((value, _new_pc, done)) => create_iterator_result(ctx, value, done),
        Err(_) => {
            if let Some(state) = gen_obj.get_generator_state_mut() {
                state.done = true;
            }
            create_iterator_result(ctx, JSValue::undefined(), true)
        }
    }
}

fn generator_return(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = args.get(0).copied().unwrap_or(JSValue::undefined());

    if !this.is_object() {
        return create_iterator_result(ctx, JSValue::undefined(), true);
    }

    let gen_obj = this.as_object_mut();

    if let Some(state) = gen_obj.get_generator_state_mut() {
        state.done = true;
    }

    let value = args.get(1).copied().unwrap_or(JSValue::undefined());
    create_iterator_result(ctx, value, true)
}

fn generator_symbol_iterator(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    args.get(0).copied().unwrap_or(JSValue::undefined())
}

fn async_generator_next(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = args.get(0).copied().unwrap_or(JSValue::undefined());

    if !this.is_object() {
        return ctx.call_builtin("promise_reject", &[JSValue::undefined()]);
    }

    let gen_obj = this.as_object_mut();

    if !gen_obj.is_generator() {
        let result = create_iterator_result(ctx, JSValue::undefined(), true);
        return ctx.call_builtin("promise_resolve", &[result]);
    }

    let is_done = gen_obj
        .get_generator_state()
        .map(|s| s.done)
        .unwrap_or(true);

    if is_done {
        let result = create_iterator_result(ctx, JSValue::undefined(), true);
        return ctx.call_builtin("promise_resolve", &[result]);
    }

    let send_value = args.get(1).copied().unwrap_or(JSValue::undefined());

    match run_generator_step(ctx, gen_obj, send_value) {
        Ok((value, _new_pc, done)) => {
            let iter_result = create_iterator_result(ctx, value, done);
            ctx.call_builtin("promise_resolve", &[iter_result])
        }
        Err(e) => {
            if let Some(state) = gen_obj.get_generator_state_mut() {
                state.done = true;
            }
            let err_val = JSValue::new_string(ctx.intern(&e));
            ctx.call_builtin("promise_reject", &[err_val])
        }
    }
}

fn create_iterator_result(ctx: &mut JSContext, value: JSValue, done: bool) -> JSValue {
    let mut result = JSObject::new();
    result.set(ctx.intern("value"), value);
    result.set(ctx.intern("done"), JSValue::bool(done));

    let ptr = Box::into_raw(Box::new(result)) as usize;
    JSValue::new_object(ptr)
}
