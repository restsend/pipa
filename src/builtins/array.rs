use crate::builtins::string::js_to_length;
use crate::host::HostFunction;
use crate::object::array_obj::JSArrayObject;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;

fn throw_type_error(ctx: &mut JSContext, msg: &str) {
    let mut err = JSObject::new();
    if let Some(proto_ptr) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto_ptr);
    }
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(msg)));
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
}

#[inline(always)]
fn require_object_coercible(ctx: &mut JSContext, val: &JSValue) -> bool {
    if val.is_undefined() || val.is_null() {
        throw_type_error(ctx, "Cannot convert undefined or null to object");
        return false;
    }
    true
}

#[inline(always)]
fn array_get(obj: &JSObject, index: usize, ctx: &mut JSContext) -> Option<crate::value::JSValue> {
    if obj.is_array() {
        let ptr = obj as *const JSObject as usize;
        if obj.is_dense_array() {
            let arr = unsafe { &*(ptr as *const JSArrayObject) };
            if let Some(val) = arr.get(index) {
                return Some(val);
            }
        }
    }

    if let Some(val) = obj.get_indexed(index) {
        return Some(val);
    }
    let key = ctx.int_atom_mut(index);
    obj.get(key)
}
use crate::value::JSValue;

fn safe_array_len(obj: &JSObject, len_atom: crate::runtime::atom::Atom) -> u64 {
    obj.get(len_atom)
        .as_ref()
        .map(|v| js_to_length(v))
        .unwrap_or(0)
}

fn init_array_length(obj: &mut JSObject, len: i64, ctx: &mut JSContext) {
    use crate::object::object::PropertyDescriptor;
    let len_atom = ctx.common_atoms.length;
    if obj.has_own(len_atom) {
        obj.set_length(len_atom, JSValue::new_int(len));
    } else {
        obj.define_property(
            len_atom,
            PropertyDescriptor {
                value: Some(JSValue::new_int(len)),
                writable: true,
                enumerable: false,
                configurable: false,
                get: None,
                set: None,
            },
        );
    }

    obj.assign_shape_from_existing_props(ctx.shape_cache_mut());
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    debug_assert_ne!(
        unsafe { (*(ptr as *const JSObject)).gc_slot },
        u32::MAX,
        "track_function did not set gc_slot"
    );
    JSValue::new_function(ptr)
}

pub fn init_array(ctx: &mut JSContext) {
    let array_atom = ctx.common_atoms.array;

    fn set_ne(obj: &mut JSObject, key: crate::runtime::atom::Atom, val: JSValue) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let mut array_func = JSFunction::new_builtin(array_atom, 1);
    array_func.set_builtin_marker(ctx, "array_constructor");

    set_ne(
        &mut array_func.base,
        ctx.intern("isArray"),
        create_builtin_function(ctx, "array_isArray"),
    );
    set_ne(
        &mut array_func.base,
        ctx.intern("fromAsync"),
        create_builtin_function(ctx, "array_fromAsync"),
    );

    let array_ptr = Box::into_raw(Box::new(array_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(array_ptr);
    let array_value = JSValue::new_function(array_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, array_atom, array_value);
    }

    let proto_atom = ctx.intern("ArrayPrototype");
    let mut proto_obj = JSObject::new();
    set_ne(
        &mut proto_obj,
        ctx.intern("push"),
        create_builtin_function(ctx, "array_push"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("pop"),
        create_builtin_function(ctx, "array_pop"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("shift"),
        create_builtin_function(ctx, "array_shift"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("unshift"),
        create_builtin_function(ctx, "array_unshift"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("concat"),
        create_builtin_function(ctx, "array_concat"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("slice"),
        create_builtin_function(ctx, "array_slice"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("indexOf"),
        create_builtin_function(ctx, "array_indexOf"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("includes"),
        create_builtin_function(ctx, "array_includes"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("join"),
        create_builtin_function(ctx, "array_join"),
    );
    init_array_length(&mut proto_obj, 0, ctx);

    set_ne(
        &mut proto_obj,
        ctx.intern("forEach"),
        create_builtin_function(ctx, "array_forEach"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("map"),
        create_builtin_function(ctx, "array_map"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("filter"),
        create_builtin_function(ctx, "array_filter"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("reduce"),
        create_builtin_function(ctx, "array_reduce"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("every"),
        create_builtin_function(ctx, "array_every"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("some"),
        create_builtin_function(ctx, "array_some"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("find"),
        create_builtin_function(ctx, "array_find"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("findIndex"),
        create_builtin_function(ctx, "array_findIndex"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("reduceRight"),
        create_builtin_function(ctx, "array_reduceRight"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("sort"),
        create_builtin_function(ctx, "array_sort"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("reverse"),
        create_builtin_function(ctx, "array_reverse"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("fill"),
        create_builtin_function(ctx, "array_fill"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("splice"),
        create_builtin_function(ctx, "array_splice"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("flat"),
        create_builtin_function(ctx, "array_flat"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("flatMap"),
        create_builtin_function(ctx, "array_flatMap"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("findLast"),
        create_builtin_function(ctx, "array_findLast"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("findLastIndex"),
        create_builtin_function(ctx, "array_findLastIndex"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("at"),
        create_builtin_function(ctx, "array_at"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("toSorted"),
        create_builtin_function(ctx, "array_toSorted"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("toReversed"),
        create_builtin_function(ctx, "array_toReversed"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("with"),
        create_builtin_function(ctx, "array_with"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("lastIndexOf"),
        create_builtin_function(ctx, "array_lastIndexOf"),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("toSpliced"),
        create_builtin_function(ctx, "array_toSpliced"),
    );

    set_ne(&mut proto_obj, ctx.common_atoms.constructor, array_value);

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);

    ctx.set_array_prototype(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);

    let array_func_ref = array_value.as_function_mut();
    array_func_ref
        .base
        .set(ctx.common_atoms.prototype, proto_value);

    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(proto_atom, proto_value);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin("array_push", HostFunction::method("push", 1, array_push));
    ctx.register_builtin("array_pop", HostFunction::method("pop", 0, array_pop));
    ctx.register_builtin("array_shift", HostFunction::method("shift", 0, array_shift));
    ctx.register_builtin(
        "array_unshift",
        HostFunction::method("unshift", 1, array_unshift),
    );
    ctx.register_builtin(
        "array_concat",
        HostFunction::method("concat", 1, array_concat),
    );
    ctx.register_builtin("array_slice", HostFunction::method("slice", 2, array_slice));
    ctx.register_builtin(
        "array_indexOf",
        HostFunction::method("indexOf", 1, array_index_of),
    );
    ctx.register_builtin(
        "array_includes",
        HostFunction::method("includes", 1, array_includes),
    );
    ctx.register_builtin("array_join", HostFunction::method("join", 1, array_join));
    ctx.register_builtin(
        "array_isArray",
        HostFunction::new("isArray", 1, array_is_array),
    );
    ctx.register_builtin(
        "array_fromAsync",
        HostFunction::new("fromAsync", 1, array_from_async),
    );

    ctx.register_builtin(
        "array_forEach",
        HostFunction::method("forEach", 1, array_for_each),
    );
    ctx.register_builtin("array_map", HostFunction::method("map", 1, array_map));
    ctx.register_builtin(
        "array_filter",
        HostFunction::method("filter", 1, array_filter),
    );
    ctx.register_builtin(
        "array_reduce",
        HostFunction::method("reduce", 2, array_reduce),
    );
    ctx.register_builtin("array_every", HostFunction::method("every", 1, array_every));
    ctx.register_builtin("array_some", HostFunction::method("some", 1, array_some));
    ctx.register_builtin("array_find", HostFunction::method("find", 1, array_find));
    ctx.register_builtin(
        "array_findIndex",
        HostFunction::method("findIndex", 1, array_find_index),
    );
    ctx.register_builtin(
        "array_reduceRight",
        HostFunction::method("reduceRight", 2, array_reduce_right),
    );
    ctx.register_builtin("array_sort", HostFunction::method("sort", 1, array_sort));
    ctx.register_builtin(
        "array_reverse",
        HostFunction::method("reverse", 0, array_reverse),
    );
    ctx.register_builtin("array_fill", HostFunction::method("fill", 1, array_fill));
    ctx.register_builtin(
        "array_splice",
        HostFunction::method("splice", 2, array_splice),
    );
    ctx.register_builtin("array_flat", HostFunction::method("flat", 0, array_flat));
    ctx.register_builtin(
        "array_flatMap",
        HostFunction::method("flatMap", 1, array_flat_map),
    );
    ctx.register_builtin(
        "array_findLast",
        HostFunction::method("findLast", 1, array_find_last),
    );
    ctx.register_builtin(
        "array_findLastIndex",
        HostFunction::method("findLastIndex", 1, array_find_last_index),
    );
    ctx.register_builtin("array_at", HostFunction::method("at", 1, array_at));
    ctx.register_builtin(
        "array_toSorted",
        HostFunction::method("toSorted", 1, array_to_sorted),
    );
    ctx.register_builtin(
        "array_toReversed",
        HostFunction::method("toReversed", 0, array_to_reversed),
    );
    ctx.register_builtin("array_with", HostFunction::method("with", 2, array_with));
    ctx.register_builtin(
        "array_lastIndexOf",
        HostFunction::method("lastIndexOf", 1, array_last_index_of),
    );
    ctx.register_builtin(
        "array_toSpliced",
        HostFunction::method("toSpliced", 3, array_to_spliced),
    );
    ctx.register_builtin(
        "array_constructor",
        HostFunction::ctor("Array", 1, array_constructor),
    );
}

fn new_jsarray_with_proto(ctx: &mut JSContext) -> JSArrayObject {
    let mut arr = JSArrayObject::new();
    if let Some(proto_ptr) = ctx.get_array_prototype() {
        arr.header.set_prototype_raw(proto_ptr);
    }
    arr
}

fn alloc_jsarray(arr: JSArrayObject, ctx: &mut JSContext) -> JSValue {
    let ptr = Box::into_raw(Box::new(arr)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_array(ptr);
    JSValue::new_object(ptr)
}

pub fn array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut arr = new_jsarray_with_proto(ctx);
        init_array_length(&mut arr.header, 0, ctx);
        return alloc_jsarray(arr, ctx);
    }

    if args.len() == 1 && args[0].is_int() {
        let len = args[0].get_int();
        let mut arr = if len > 0 {
            JSArrayObject::with_length(len as usize)
        } else {
            JSArrayObject::new()
        };
        if let Some(proto_ptr) = ctx.get_array_prototype() {
            arr.header.set_prototype_raw(proto_ptr);
        }
        if len < 0 {
            init_array_length(&mut arr.header, 0, ctx);
        } else {
            init_array_length(&mut arr.header, len, ctx);
        }
        return alloc_jsarray(arr, ctx);
    }

    let mut arr = new_jsarray_with_proto(ctx);
    for arg in args.iter() {
        arr.push(*arg);
    }
    init_array_length(&mut arr.header, args.len() as i64, ctx);
    alloc_jsarray(arr, ctx)
}

fn array_push(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }
    let this = &args[0];
    if !this.is_object() {
        return JSValue::new_int(0);
    }

    let length_atom = ctx.common_atoms.length;
    let ptr = this.get_ptr();

    if this.as_object().is_dense_array() {
        let arr = unsafe { &mut *(ptr as *mut JSArrayObject) };
        let current_len = arr.len() as u32;
        for arg in args.iter().skip(1) {
            arr.push(arg.clone());
        }
        let new_len = current_len + (args.len() - 1) as u32;
        arr.header.set_length_ic(
            length_atom,
            JSValue::new_int(new_len as i64),
            ctx.shape_cache_mut(),
        );
        return JSValue::new_int(new_len as i64);
    }

    let obj = unsafe { &mut *(ptr as *mut JSObject) };
    let current_len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as u32 } else { 0 }
    } else {
        0
    };

    for (i, arg) in args.iter().skip(1).enumerate() {
        let idx = current_len as usize + i;
        let key = ctx.int_atom_mut(idx);
        obj.set(key, arg.clone());
    }

    let new_len = current_len + (args.len() - 1) as u32;
    obj.set_length(length_atom, JSValue::new_int(new_len as i64));
    JSValue::new_int(new_len as i64)
}

fn array_pop(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = &args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }

    let length_atom = ctx.common_atoms.length;
    let ptr = this.get_ptr();

    if this.as_object().is_dense_array() {
        let arr = unsafe { &mut *(ptr as *mut JSArrayObject) };
        let current_len = arr.len();
        if current_len == 0 {
            return JSValue::undefined();
        }
        let result = arr
            .elements()
            .last()
            .copied()
            .unwrap_or_else(JSValue::undefined);
        arr.elements_mut().pop();
        arr.header
            .set_length(length_atom, JSValue::new_int((current_len - 1) as i64));
        return result;
    }

    let obj = unsafe { &mut *(ptr as *mut JSObject) };
    let current_len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as u32 } else { 0 }
    } else {
        0
    };

    if current_len == 0 {
        return JSValue::undefined();
    }

    let idx = (current_len - 1) as usize;
    let key = ctx.int_atom_mut(idx);
    let result = obj.get(key).unwrap_or_else(JSValue::undefined);
    obj.delete(key);
    obj.set_length(length_atom, JSValue::new_int(idx as i64));
    result
}

fn array_shift(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = &args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let length_atom = ctx.common_atoms.length;
    let ptr = this.get_ptr();

    if this.as_object().is_dense_array() {
        let arr = unsafe { &mut *(ptr as *mut JSArrayObject) };
        let current_len = arr.len();
        if current_len == 0 {
            return JSValue::undefined();
        }
        let result = arr.get(0).unwrap_or_else(JSValue::undefined);
        arr.elements_mut().remove(0);
        arr.header
            .set_length(length_atom, JSValue::new_int((current_len - 1) as i64));
        return result;
    }

    let obj = unsafe { &mut *(ptr as *mut JSObject) };
    let current_len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as u32 } else { 0 }
    } else {
        0
    };

    if current_len == 0 {
        return JSValue::undefined();
    }

    let first_key = ctx.common_atoms.n0;
    let result = obj.get(first_key).unwrap_or_else(JSValue::undefined);

    for i in 1..current_len {
        let old_key = ctx.int_atom_mut(i as usize);
        let new_key = ctx.int_atom_mut(i as usize - 1);
        if let Some(val) = obj.get(old_key) {
            obj.set(new_key, val);
        }
        obj.delete(old_key);
    }

    let last_key = ctx.int_atom_mut(current_len as usize - 1);
    obj.delete(last_key);
    obj.set_length(length_atom, JSValue::new_int((current_len - 1) as i64));
    result
}

fn array_unshift(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }
    let this = &args[0];
    if !this.is_object() {
        return JSValue::new_int(0);
    }

    let length_atom = ctx.common_atoms.length;
    let add_count = (args.len() - 1) as usize;
    let ptr = this.get_ptr();

    if this.as_object().is_dense_array() {
        let arr = unsafe { &mut *(ptr as *mut JSArrayObject) };

        let items: Vec<JSValue> = args.iter().skip(1).cloned().collect();
        let mut new_elements = items;
        new_elements.extend(arr.elements().iter().copied());
        arr.set_elements(new_elements);
        let new_len = arr.len();
        arr.header
            .set_length(length_atom, JSValue::new_int(new_len as i64));
        return JSValue::new_int(new_len as i64);
    }

    let obj = unsafe { &mut *(ptr as *mut JSObject) };
    let current_len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as u32 } else { 0 }
    } else {
        0
    };

    for i in (0..current_len).rev() {
        let old_key = ctx.int_atom_mut(i as usize);
        let new_key = ctx.int_atom_mut(i as usize + add_count);
        if let Some(val) = obj.get(old_key) {
            obj.set(new_key, val);
        }
        obj.delete(old_key);
    }

    for (i, arg) in args.iter().skip(1).enumerate() {
        let key = ctx.int_atom_mut(i);
        obj.set(key, arg.clone());
    }

    let new_len = current_len + add_count as u32;
    obj.set_length(length_atom, JSValue::new_int(new_len as i64));
    JSValue::new_int(new_len as i64)
}

fn array_concat(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut result = new_jsarray_with_proto(ctx);
        result
            .header
            .set_length(ctx.common_atoms.length, JSValue::new_int(0));
        return alloc_jsarray(result, ctx);
    }

    let this = &args[0];
    let mut result = new_jsarray_with_proto(ctx);

    let length_atom = ctx.common_atoms.length;

    if this.is_object() {
        let obj = this.as_object();
        let this_len = if let Some(l) = obj.get(length_atom) {
            if l.is_int() { l.get_int() as u32 } else { 0 }
        } else {
            0
        };

        for i in 0..this_len {
            if let Some(val) = array_get(obj, i as usize, ctx) {
                result.push(val);
            }
        }
    }

    for arg in args.iter().skip(1) {
        if arg.is_object() {
            let obj = arg.as_object();
            let arg_len = if let Some(l) = obj.get(length_atom) {
                if l.is_int() { l.get_int() as u32 } else { 0 }
            } else {
                0
            };

            for i in 0..arg_len {
                if let Some(val) = array_get(obj, i as usize, ctx) {
                    result.push(val);
                }
            }
        } else {
            result.push(arg.clone());
        }
    }

    let result_len = result.len();
    result
        .header
        .set_length(length_atom, JSValue::new_int(result_len as i64));
    alloc_jsarray(result, ctx)
}

fn array_slice(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut result = new_jsarray_with_proto(ctx);
    let length_atom = ctx.common_atoms.length;

    if args.is_empty() {
        result.header.set_length(length_atom, JSValue::new_int(0));
        return alloc_jsarray(result, ctx);
    }

    let this = &args[0];
    if !this.is_object() {
        result.header.set_length(length_atom, JSValue::new_int(0));
        return alloc_jsarray(result, ctx);
    }

    let obj = this.as_object();

    let len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as i32 } else { 0 }
    } else {
        0
    };

    let start = if args.len() > 1 {
        let s = args[1].get_int() as i32;
        if s < 0 { (len + s).max(0) } else { s.min(len) }
    } else {
        0
    };

    let end = if args.len() > 2 {
        let e = args[2].get_int() as i32;
        if e < 0 { (len + e).max(0) } else { e.min(len) }
    } else {
        len
    };

    for i in start..end {
        if let Some(val) = array_get(obj, i as usize, ctx) {
            result.push(val);
        }
    }

    result
        .header
        .set_length(length_atom, JSValue::new_int(result.len() as i64));
    alloc_jsarray(result, ctx)
}

fn array_index_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_int(-1);
    }

    let this = &args[0];
    let search = &args[1];

    if !this.is_object() {
        return JSValue::new_int(-1);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as u64 } else { 0 }
    } else {
        0
    };

    let from_index_f = if args.len() > 2 {
        args[2].to_number()
    } else {
        0.0
    };

    let start = if from_index_f.is_nan() {
        0u64
    } else if from_index_f < 0.0 {
        ((len as f64) + from_index_f).max(0.0) as u64
    } else {
        (from_index_f as u64).min(len)
    };

    if start >= len {
        return JSValue::new_int(-1);
    }

    if len - start > 10_000_000 {
        if obj.is_dense_array() {
            let ptr = this.get_ptr();
            let dense_arr = unsafe { &*(ptr as *const crate::object::array_obj::JSArrayObject) };
            let dense_len = dense_arr.elements.len() as u64;
            for i in start..dense_len.min(len) {
                if let Some(val) = dense_arr.elements.get(i as usize) {
                    if val.strict_eq(search) {
                        return JSValue::new_int(i as i64);
                    }
                }
            }
        } else {
            for slot in obj.iter_props() {
                let s = ctx.get_atom_str(slot.atom);
                if let Ok(idx) = s.parse::<u64>() {
                    if idx >= start && idx < len && slot.value.strict_eq(search) {
                        return JSValue::new_int(idx as i64);
                    }
                }
            }
        }
    } else {
        for i in start..len {
            if let Some(val) = array_get(obj, i as usize, ctx) {
                if val.strict_eq(search) {
                    return JSValue::new_int(i as i64);
                }
            }
        }
    }

    JSValue::new_int(-1)
}

fn array_includes(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }

    let this = &args[0];
    let search = &args[1];

    if !this.is_object() {
        return JSValue::bool(false);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = if let Some(l) = obj.get(length_atom) {
        if l.is_int() { l.get_int() as u64 } else { 0 }
    } else {
        0
    };

    let from_index_f = if args.len() > 2 {
        args[2].to_number()
    } else {
        0.0
    };

    let start = if from_index_f.is_nan() {
        0u64
    } else if from_index_f < 0.0 {
        ((len as f64) + from_index_f).max(0.0) as u64
    } else {
        (from_index_f as u64).min(len)
    };

    if start >= len {
        return JSValue::bool(false);
    }

    if len - start > 10_000_000 {
        if obj.is_dense_array() {
            let ptr = this.get_ptr();
            let dense_arr = unsafe { &*(ptr as *const crate::object::array_obj::JSArrayObject) };
            let dense_len = dense_arr.elements.len() as u64;
            for i in start..dense_len.min(len) {
                if let Some(val) = dense_arr.elements.get(i as usize) {
                    if val_same_value_zero(val, search) {
                        return JSValue::bool(true);
                    }
                }
            }
        } else {
            for slot in obj.iter_props() {
                let s = ctx.get_atom_str(slot.atom);
                if let Ok(idx) = s.parse::<u64>() {
                    if idx >= start && idx < len && val_same_value_zero(&slot.value, search) {
                        return JSValue::bool(true);
                    }
                }
            }
        }
    } else {
        for i in start..len {
            if let Some(val) = array_get(obj, i as usize, ctx) {
                if val_same_value_zero(&val, search) {
                    return JSValue::bool(true);
                }
            }
        }
    }

    JSValue::bool(false)
}

fn val_same_value_zero(a: &JSValue, b: &JSValue) -> bool {
    if a.is_float() && b.is_float() {
        let af = a.get_float();
        let bf = b.get_float();
        if af.is_nan() && bf.is_nan() {
            return true;
        }
        af.to_bits() == bf.to_bits()
    } else {
        a.strict_eq(b)
    }
}

fn array_join(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }

    let this = &args[0];
    if !this.is_object() {
        return JSValue::new_string(ctx.intern(""));
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    let separator = if args.len() > 1 {
        let sep = &args[1];
        if sep.is_undefined() {
            ",".to_string()
        } else if sep.is_string() {
            ctx.get_atom_str(sep.get_atom()).to_string()
        } else if sep.is_bool() {
            sep.get_bool().to_string()
        } else if sep.is_int() {
            sep.get_int().to_string()
        } else if sep.is_float() {
            let f = sep.get_float();
            if f.is_nan() {
                "NaN".to_string()
            } else if f.is_infinite() {
                "Infinity".to_string()
            } else {
                f.to_string()
            }
        } else if sep.is_null() {
            "null".to_string()
        } else {
            ",".to_string()
        }
    } else {
        ",".to_string()
    };

    let mut parts = Vec::new();
    for i in 0..len {
        if let Some(val) = array_get(obj, i as usize, ctx) {
            if val.is_undefined() || val.is_null() {
                parts.push(String::new());
            } else if val.is_string() {
                parts.push(ctx.get_atom_str(val.get_atom()).to_string());
            } else if val.is_int() {
                parts.push(val.get_int().to_string());
            } else if val.is_float() {
                parts.push(val.get_float().to_string());
            } else if val.is_bool() {
                parts.push(val.get_bool().to_string());
            } else {
                parts.push(String::new());
            }
        } else {
            parts.push(String::new());
        }
    }

    JSValue::new_string(ctx.intern(&parts.join(&separator)))
}

fn array_is_array(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }

    let arg = &args[0];
    if arg.is_object() {
        let obj = arg.as_object();
        JSValue::bool(obj.is_array())
    } else {
        JSValue::bool(false)
    }
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
        Err("call_callback: VM not available".to_string())
    }
}

fn array_for_each(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::undefined();
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            let _ = call_callback(ctx, callback, &callback_args);
        }
    }

    JSValue::undefined()
}

fn array_map(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        let mut result = new_jsarray_with_proto(ctx);
        result
            .header
            .set_length(ctx.common_atoms.length, JSValue::new_int(0));
        return alloc_jsarray(result, ctx);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    let mut result = new_jsarray_with_proto(ctx);

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            if let Ok(mapped_value) = call_callback(ctx, callback, &callback_args) {
                result.push(mapped_value);
            }
        }
    }

    result
        .header
        .set_length(length_atom, JSValue::new_int(result.len() as i64));
    alloc_jsarray(result, ctx)
}

fn array_filter(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        let mut result = new_jsarray_with_proto(ctx);
        result
            .header
            .set_length(ctx.common_atoms.length, JSValue::new_int(0));
        return alloc_jsarray(result, ctx);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    let mut result = new_jsarray_with_proto(ctx);

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            if let Ok(test_result) = call_callback(ctx, callback, &callback_args) {
                if test_result.is_truthy() {
                    result.push(value);
                }
            }
        }
    }

    result
        .header
        .set_length(length_atom, JSValue::new_int(result.len() as i64));
    alloc_jsarray(result, ctx)
}

fn array_reduce(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::undefined();
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    if len == 0 {
        if args.len() > 2 {
            return args[2];
        }
        throw_type_error(ctx, "Reduce of empty array with no initial value");
        return JSValue::undefined();
    }

    let mut accumulator: JSValue;
    let mut start_index: usize = 0;

    if args.len() > 2 {
        accumulator = args[2];
    } else {
        accumulator = array_get(obj, 0, ctx).unwrap_or_else(JSValue::undefined);
        start_index = 1;
    }

    for i in start_index..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![accumulator, value, JSValue::new_int(i as i64), this];
            if let Ok(result) = call_callback(ctx, callback, &callback_args) {
                accumulator = result;
            }
        }
    }

    accumulator
}

fn array_every(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::bool(true);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            if let Ok(result) = call_callback(ctx, callback, &callback_args) {
                if !result.is_truthy() {
                    return JSValue::bool(false);
                }
            }
        }
    }

    JSValue::bool(true)
}

fn array_some(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::bool(false);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            if let Ok(result) = call_callback(ctx, callback, &callback_args) {
                if result.is_truthy() {
                    return JSValue::bool(true);
                }
            }
        }
    }

    JSValue::bool(false)
}

fn array_find(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::undefined();
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            if let Ok(result) = call_callback(ctx, callback, &callback_args) {
                if result.is_truthy() {
                    return value;
                }
            }
        }
    }

    JSValue::undefined()
}

fn array_find_index(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::new_int(-1);
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    for i in 0..len {
        if let Some(value) = array_get(obj, i as usize, ctx) {
            let callback_args = vec![value, JSValue::new_int(i as i64), this];
            if let Ok(result) = call_callback(ctx, callback, &callback_args) {
                if result.is_truthy() {
                    return JSValue::new_int(i as i64);
                }
            }
        }
    }

    JSValue::new_int(-1)
}

fn array_reduce_right(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }

    if !this.is_object() {
        return JSValue::undefined();
    }

    let obj = this.as_object();

    let length_atom = ctx.common_atoms.length;
    let len = safe_array_len(obj, length_atom) as usize;

    if len == 0 {
        if args.len() > 2 {
            return args[2];
        }
        return JSValue::undefined();
    }

    let mut accumulator: JSValue;
    let mut end_index: i64;

    if args.len() > 2 {
        accumulator = args[2];
        end_index = len as i64 - 1;
    } else {
        accumulator = array_get(obj, (len - 1) as usize, ctx).unwrap_or_else(JSValue::undefined);
        end_index = len as i64 - 2;
    }

    while end_index >= 0 {
        if let Some(value) = array_get(obj, end_index as usize, ctx) {
            let callback_args = vec![accumulator, value, JSValue::new_int(end_index), this];
            if let Ok(result) = call_callback(ctx, callback, &callback_args) {
                accumulator = result;
            }
        }
        end_index -= 1;
    }

    accumulator
}

fn val_to_str(ctx: &mut JSContext, v: JSValue) -> String {
    if v.is_int() {
        format!("{}", v.get_int())
    } else if v.is_float() {
        format!("{}", v.get_float())
    } else if v.is_string() {
        ctx.get_atom_str(v.get_atom()).to_string()
    } else if v.is_bool() {
        v.get_bool().to_string()
    } else if v.is_null() {
        "null".to_string()
    } else {
        "undefined".to_string()
    }
}

fn array_sort(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return this;
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    let mut elements: Vec<JSValue> = (0..len)
        .map(|i| {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        })
        .collect();

    let cmp_fn = args.get(1).copied().filter(|v| v.is_function());

    let n = elements.len();
    for i in 1..n {
        let mut j = i;
        while j > 0 {
            let should_swap = if let Some(func) = cmp_fn {
                match call_callback(ctx, func, &[elements[j - 1], elements[j]]) {
                    Ok(r) => r.get_float() > 0.0 || (r.is_int() && r.get_int() > 0),
                    Err(_) => false,
                }
            } else {
                let a_str = val_to_str(ctx, elements[j - 1]);
                let b_str = val_to_str(ctx, elements[j]);
                a_str > b_str
            };
            if should_swap {
                elements.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }

    let arr = this.as_object_mut();
    if this.as_object().is_dense_array() {
        let arr_obj = unsafe { &mut *(this.get_ptr() as *mut JSArrayObject) };
        arr_obj.set_elements(elements);
    } else {
        if arr.has_dense_storage() {
            arr.set_array_elements(elements.clone());
        }
        for (i, val) in elements.into_iter().enumerate() {
            let key = ctx.int_atom_mut(i);
            arr.set(key, val);
        }
    }
    this
}

fn array_reverse(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return this;
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        arr.get(len_atom)
            .as_ref()
            .map(|v| js_to_length(v))
            .unwrap_or(0)
    };
    if len > 10_000_000 {
        return this;
    }
    let mut elements: Vec<JSValue> = (0..len)
        .map(|i| {
            let arr = this.as_object();
            array_get(arr, i as usize, ctx).unwrap_or(JSValue::undefined())
        })
        .collect();
    elements.reverse();
    let arr = this.as_object_mut();
    if this.as_object().is_dense_array() {
        let arr_obj = unsafe { &mut *(this.get_ptr() as *mut JSArrayObject) };
        arr_obj.set_elements(elements);
    } else {
        if arr.has_dense_storage() {
            arr.set_array_elements(elements.clone());
        }
        for (i, val) in elements.into_iter().enumerate() {
            let key = ctx.int_atom_mut(i);
            arr.set(key, val);
        }
    }
    this
}

fn array_fill(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return this;
    }
    let value = args.get(1).copied().unwrap_or(JSValue::undefined());
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    let start = args
        .get(2)
        .map(|v| {
            let i = v.get_int() as isize;
            if i < 0 {
                (len as isize + i).max(0) as usize
            } else {
                (i as usize).min(len)
            }
        })
        .unwrap_or(0);
    let end = args
        .get(3)
        .map(|v| {
            let i = v.get_int() as isize;
            if i < 0 {
                (len as isize + i).max(0) as usize
            } else {
                (i as usize).min(len)
            }
        })
        .unwrap_or(len);
    let arr = this.as_object_mut();
    for i in start..end {
        let key = ctx.int_atom_mut(i);
        arr.set(key, value);
    }
    this
}

fn array_splice(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        arr.get(len_atom)
            .as_ref()
            .map(|v| js_to_length(v))
            .unwrap_or(0)
    };
    let start = args
        .get(1)
        .map(|v| {
            let n = v.to_number();
            let i = if n < 0.0 {
                (len as f64 + n).max(0.0) as u64
            } else {
                n.min(len as f64) as u64
            };
            i.min(len)
        })
        .unwrap_or(0);
    let delete_count = args
        .get(2)
        .map(|v| (js_to_length(v)).min(len.saturating_sub(start)))
        .unwrap_or(len.saturating_sub(start));
    let insert_items: Vec<JSValue> = args.iter().skip(3).copied().collect();

    let new_len = len - delete_count + insert_items.len() as u64;
    let mut removed: Vec<JSValue> = Vec::with_capacity(delete_count.min(1024) as usize);

    for i in 0..delete_count {
        let arr = this.as_object();
        removed.push(array_get(arr, (start + i) as usize, ctx).unwrap_or(JSValue::undefined()));
    }

    let shift = insert_items.len() as i64 - delete_count as i64;

    if shift < 0 {
        for i in start..(len - delete_count) {
            let src_idx = (start + delete_count + (i - start)) as usize;
            let dst_idx = (start + insert_items.len() as u64 + (i - start)) as usize;
            let val = {
                let arr = this.as_object();
                array_get(arr, src_idx, ctx).unwrap_or(JSValue::undefined())
            };
            let arr = this.as_object_mut();
            arr.set(ctx.int_atom_mut(dst_idx), val);
        }
        for i in new_len..len {
            let arr = this.as_object_mut();
            arr.delete(ctx.int_atom_mut(i as usize));
        }
    } else if shift > 0 {
        for i in (start..(len - delete_count)).rev() {
            let src_idx = (start + delete_count + (i - start)) as usize;
            let dst_idx = (start + insert_items.len() as u64 + (i - start)) as usize;
            let val = {
                let arr = this.as_object();
                array_get(arr, src_idx, ctx).unwrap_or(JSValue::undefined())
            };
            let arr = this.as_object_mut();
            arr.set(ctx.int_atom_mut(dst_idx), val);
        }
    }

    for (i, item) in insert_items.iter().enumerate() {
        let arr = this.as_object_mut();
        arr.set(ctx.int_atom_mut((start + i as u64) as usize), *item);
    }

    let arr = this.as_object_mut();
    arr.set_length(len_atom, JSValue::new_int(new_len as i64));

    let mut removed_arr = new_jsarray_with_proto(ctx);
    for v in removed.iter() {
        removed_arr.push(*v);
    }
    removed_arr
        .header
        .set_length(len_atom, JSValue::new_int(removed_arr.len() as i64));

    alloc_jsarray(removed_arr, ctx)
}

fn array_to_spliced(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    let start = args
        .get(1)
        .map(|v| {
            let i = v.get_int() as isize;
            if i < 0 {
                (len as isize + i).max(0) as usize
            } else {
                (i as usize).min(len)
            }
        })
        .unwrap_or(0);
    let delete_count = args
        .get(2)
        .map(|v| (v.get_int() as usize).min(len - start))
        .unwrap_or(len - start);
    let insert_items: Vec<JSValue> = args.iter().skip(3).copied().collect();

    let mut elements: Vec<JSValue> = (0..len)
        .map(|i| {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        })
        .collect();

    elements.splice(start..start + delete_count, insert_items);

    let mut result_arr = new_jsarray_with_proto(ctx);
    for v in elements.iter() {
        result_arr.push(*v);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    alloc_jsarray(result_arr, ctx)
}

fn array_flat(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let depth = args.get(1).map(|v| v.get_int() as usize).unwrap_or(1);

    fn flatten(ctx: &mut JSContext, arr_val: JSValue, depth: usize, result: &mut Vec<JSValue>) {
        if !arr_val.is_object() {
            result.push(arr_val);
            return;
        }
        let len_atom = ctx.common_atoms.length;
        let arr = arr_val.as_object();
        if !arr.is_array() {
            result.push(arr_val);
            return;
        }
        let len = safe_array_len(&arr, len_atom) as usize;
        let elements: Vec<JSValue> = (0..len)
            .map(|i| array_get(arr, i, ctx).unwrap_or(JSValue::undefined()))
            .collect();
        for el in elements {
            if depth > 0 && el.is_object() {
                let el_obj = el.as_object();
                if el_obj.is_array() {
                    flatten(ctx, el, depth - 1, result);
                    continue;
                }
            }
            result.push(el);
        }
    }

    let mut flat_elements: Vec<JSValue> = Vec::new();
    flatten(ctx, this, depth, &mut flat_elements);

    let len_atom = ctx.common_atoms.length;
    let mut result_arr = new_jsarray_with_proto(ctx);
    for v in flat_elements.iter() {
        result_arr.push(*v);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    alloc_jsarray(result_arr, ctx)
}

fn array_flat_map(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let this = args[0];
    let callback = args[1];
    if !this.is_object() || !callback.is_function() {
        return JSValue::undefined();
    }

    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };

    let mut result_elements: Vec<JSValue> = Vec::new();
    for i in 0..len {
        let el = {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        };
        let idx_val = JSValue::new_int(i as i64);
        match call_callback(ctx, callback, &[el, idx_val, this]) {
            Ok(mapped) => {
                if mapped.is_object() {
                    let mapped_obj = mapped.as_object();
                    if mapped_obj.is_array() {
                        let mlen = mapped_obj
                            .get(len_atom)
                            .map(|v| v.get_int() as usize)
                            .unwrap_or(0);
                        let mel_vec: Vec<JSValue> = (0..mlen)
                            .map(|j| array_get(mapped_obj, j, ctx).unwrap_or(JSValue::undefined()))
                            .collect();
                        result_elements.extend(mel_vec);
                        continue;
                    }
                }
                result_elements.push(mapped);
            }
            Err(_) => {}
        }
    }

    let mut result_arr = new_jsarray_with_proto(ctx);
    for v in result_elements.iter() {
        result_arr.push(*v);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    alloc_jsarray(result_arr, ctx)
}

fn array_find_last(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());

    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    for i in (0..len).rev() {
        let el = {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        };
        let idx_val = JSValue::new_int(i as i64);
        if let Ok(result) = call_callback(ctx, callback, &[el, idx_val, this]) {
            if result.is_truthy() {
                return el;
            }
        }
    }
    JSValue::undefined()
}
fn array_find_last_index(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() { JSValue::undefined() } else { args[0] };

    if !require_object_coercible(ctx, &this) {
        return JSValue::undefined();
    }

    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());

    if !this.is_object() {
        return JSValue::new_int(-1);
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    for i in (0..len).rev() {
        let el = {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        };
        let idx_val = JSValue::new_int(i as i64);
        if let Ok(result) = call_callback(ctx, callback, &[el, idx_val, this]) {
            if result.is_truthy() {
                return JSValue::new_int(i as i64);
            }
        }
    }
    JSValue::new_int(-1)
}

fn array_at(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let arr = this.as_object();
    let len = safe_array_len(&arr, len_atom) as usize;
    let idx = args.get(1).map(|v| v.get_int()).unwrap_or(0);
    let actual_idx = if idx < 0 { len as i64 + idx } else { idx };
    if actual_idx < 0 || actual_idx as usize >= len {
        return JSValue::undefined();
    }
    array_get(arr, actual_idx as usize, ctx).unwrap_or(JSValue::undefined())
}

fn array_to_sorted(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    let mut elements: Vec<JSValue> = (0..len)
        .map(|i| {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        })
        .collect();

    let cmp_fn = args.get(1).copied().filter(|v| v.is_function());
    let n = elements.len();
    for i in 1..n {
        let mut j = i;
        while j > 0 {
            let should_swap = if let Some(func) = cmp_fn {
                match call_callback(ctx, func, &[elements[j - 1], elements[j]]) {
                    Ok(r) => r.get_float() > 0.0 || (r.is_int() && r.get_int() > 0),
                    Err(_) => false,
                }
            } else {
                let a_str = val_to_str(ctx, elements[j - 1]);
                let b_str = val_to_str(ctx, elements[j]);
                a_str > b_str
            };
            if should_swap {
                elements.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }

    let mut result_arr = new_jsarray_with_proto(ctx);
    for v in elements.iter() {
        result_arr.push(*v);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    alloc_jsarray(result_arr, ctx)
}

fn array_to_reversed(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    let mut elements: Vec<JSValue> = (0..len)
        .map(|i| {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        })
        .collect();
    elements.reverse();
    let mut result_arr = new_jsarray_with_proto(ctx);
    for v in elements.iter() {
        result_arr.push(*v);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    alloc_jsarray(result_arr, ctx)
}

fn array_with(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 {
        return JSValue::undefined();
    }
    let this = args[0];
    let idx = args[1].get_int();
    let value = args[2];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let len_atom = ctx.common_atoms.length;
    let len = {
        let arr = this.as_object();
        safe_array_len(&arr, len_atom) as usize
    };
    let mut elements: Vec<JSValue> = (0..len)
        .map(|i| {
            let arr = this.as_object();
            array_get(arr, i, ctx).unwrap_or(JSValue::undefined())
        })
        .collect();
    let actual_idx = if idx < 0 {
        (len as i64 + idx) as usize
    } else {
        idx as usize
    };
    if actual_idx < len {
        elements[actual_idx] = value;
    }
    let mut result_arr = new_jsarray_with_proto(ctx);
    for v in elements.iter() {
        result_arr.push(*v);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    alloc_jsarray(result_arr, ctx)
}

fn array_last_index_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_int(-1);
    }
    let this = args[0];
    let search = args[1];
    if !this.is_object() {
        return JSValue::new_int(-1);
    }
    let len_atom = ctx.common_atoms.length;
    let arr = this.as_object();
    let len = arr.get(len_atom).map(|v| v.get_int() as u64).unwrap_or(0);

    let from_index_f = if args.len() > 2 {
        args[2].to_number()
    } else {
        len.saturating_sub(1) as f64
    };

    let end = if from_index_f.is_nan() {
        0i64
    } else if from_index_f < 0.0 {
        ((len as f64) + from_index_f).max(-1.0) as i64
    } else {
        (from_index_f as u64).min(len.saturating_sub(1)) as i64
    };

    if end < 0 || len == 0 {
        return JSValue::new_int(-1);
    }

    let end_u = end as u64;

    if end_u > 10_000_000 {
        if arr.is_dense_array() {
            let ptr = this.get_ptr();
            let dense_arr = unsafe { &*(ptr as *const crate::object::array_obj::JSArrayObject) };
            let dense_len = dense_arr.elements.len() as u64;
            for i in (0..=end_u.min(dense_len.saturating_sub(1))).rev() {
                if let Some(val) = dense_arr.elements.get(i as usize) {
                    if val.strict_eq(&search) {
                        return JSValue::new_int(i as i64);
                    }
                }
            }
        } else {
            let mut candidates: Vec<u64> = Vec::new();
            for slot in arr.iter_props() {
                let s = ctx.get_atom_str(slot.atom);
                if let Ok(idx) = s.parse::<u64>() {
                    if idx <= end_u && idx < len && slot.value.strict_eq(&search) {
                        candidates.push(idx);
                    }
                }
            }
            if let Some(&max) = candidates.iter().max() {
                return JSValue::new_int(max as i64);
            }
        }
    } else {
        for i in (0..=end_u).rev() {
            if i < len {
                if let Some(el) = array_get(arr, i as usize, ctx) {
                    if el.strict_eq(&search) {
                        return JSValue::new_int(i as i64);
                    }
                }
            }
        }
    }
    JSValue::new_int(-1)
}

fn array_from_async(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return create_thenable_result(ctx, Vec::new());
    }

    let items = &args[0];

    if items.is_null() || items.is_undefined() {
        return create_thenable_result(ctx, Vec::new());
    }

    if items.is_object() {
        let obj = items.as_object();
        let then_atom = ctx.common_atoms.then;
        if obj.get(then_atom).is_some() {
            let result = vec![*items];
            return create_thenable_result(ctx, result);
        }

        let len_atom = ctx.common_atoms.length;
        if let Some(len_val) = obj.get(len_atom) {
            let len = len_val.get_int() as usize;
            let mut elements: Vec<JSValue> = Vec::with_capacity(len);
            for i in 0..len {
                if let Some(val) = array_get(obj, i, ctx) {
                    elements.push(val);
                } else {
                    elements.push(JSValue::undefined());
                }
            }
            return create_thenable_result(ctx, elements);
        }
    }

    create_thenable_result(ctx, Vec::new())
}

fn create_thenable_result(ctx: &mut JSContext, elements: Vec<JSValue>) -> JSValue {
    use crate::object::object::JSObject;

    let mut result_arr = new_jsarray_with_proto(ctx);
    let len_atom = ctx.common_atoms.length;
    for val in elements.iter() {
        result_arr.push(*val);
    }
    result_arr
        .header
        .set_length(len_atom, JSValue::new_int(result_arr.len() as i64));
    let arr_value = alloc_jsarray(result_arr, ctx);

    let mut promise = JSObject::new_promise();

    if let Some(proto_ptr) = ctx.get_promise_prototype() {
        promise.prototype = Some(proto_ptr);
    }

    let state_atom = ctx.intern("__promise_state__");
    promise.set(state_atom, JSValue::new_int(1));

    let result_atom = ctx.intern("__promise_result__");
    promise.set(result_atom, arr_value);

    let reactions_atom = ctx.intern("__promise_reactions__");
    promise.set(reactions_atom, JSValue::null());

    let promise_ptr = Box::into_raw(Box::new(promise)) as usize;
    JSValue::new_object(promise_ptr)
}
