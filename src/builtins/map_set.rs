use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn map_size(ctx: &mut JSContext, obj: &JSObject) -> usize {
    let size_atom = ctx.intern("__map_size__");
    obj.get(size_atom)
        .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
        .unwrap_or(0)
}

fn map_key_atom(ctx: &mut JSContext, i: usize) -> crate::runtime::atom::Atom {
    ctx.intern(&format!("__mk_{}__", i))
}

fn map_val_atom(ctx: &mut JSContext, i: usize) -> crate::runtime::atom::Atom {
    ctx.intern(&format!("__mv_{}__", i))
}

fn map_find(ctx: &mut JSContext, obj: &JSObject, search_key: &JSValue) -> Option<usize> {
    let size = map_size(ctx, obj);
    for i in 0..size {
        let k_atom = map_key_atom(ctx, i);
        if let Some(k) = obj.get(k_atom) {
            if k.strict_eq(search_key) {
                return Some(i);
            }
        }
    }
    None
}

fn set_size(ctx: &mut JSContext, obj: &JSObject) -> usize {
    let size_atom = ctx.intern("__set_size__");
    obj.get(size_atom)
        .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
        .unwrap_or(0)
}

fn set_val_atom(ctx: &mut JSContext, i: usize) -> crate::runtime::atom::Atom {
    ctx.intern(&format!("__sv_{}__", i))
}

fn set_find(ctx: &mut JSContext, obj: &JSObject, search: &JSValue) -> Option<usize> {
    let size = set_size(ctx, obj);
    for i in 0..size {
        let v_atom = set_val_atom(ctx, i);
        if let Some(v) = obj.get(v_atom) {
            if v.strict_eq(search) {
                return Some(i);
            }
        }
    }
    None
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

fn make_array(ctx: &mut JSContext, items: Vec<JSValue>) -> JSValue {
    let mut arr = JSObject::new_array();
    if let Some(proto_ptr) = ctx.get_array_prototype() {
        arr.prototype = Some(proto_ptr);
    }
    let len = items.len();
    for (i, v) in items.into_iter().enumerate() {
        let key = ctx.intern(&i.to_string());
        arr.set(key, v);
    }
    arr.set(ctx.common_atoms.length, JSValue::new_int(len as i64));
    JSValue::new_object(Box::into_raw(Box::new(arr)) as usize)
}

pub fn map_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut obj = JSObject::new();

    if let Some(proto_ptr) = ctx.get_map_prototype() {
        obj.prototype = Some(proto_ptr);
    }

    let size_atom = ctx.intern("__map_size__");
    obj.set(size_atom, JSValue::new_int(0));

    if let Some(iterable) = args.first() {
        if iterable.is_object() {
            let iter_obj = iterable.as_object();
            let len_atom = ctx.common_atoms.length;
            let len = iter_obj
                .get(len_atom)
                .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
                .unwrap_or(0);

            for i in 0..len {
                let idx_atom = ctx.intern(&i.to_string());
                if let Some(pair) = iter_obj.get(idx_atom) {
                    if pair.is_object() {
                        let pair_obj = pair.as_object();

                        let k_atom = ctx.intern("0");
                        let v_atom = ctx.intern("1");
                        let k = pair_obj.get(k_atom).unwrap_or_else(JSValue::undefined);
                        let v = pair_obj.get(v_atom).unwrap_or_else(JSValue::undefined);

                        let cur_size = map_size(ctx, &obj);

                        if let Some(idx) = map_find(ctx, &obj, &k) {
                            let mv_atom = map_val_atom(ctx, idx);
                            obj.set(mv_atom, v);
                        } else {
                            let mk_atom = map_key_atom(ctx, cur_size);
                            let mv_atom = map_val_atom(ctx, cur_size);
                            obj.set(mk_atom, k);
                            obj.set(mv_atom, v);
                            obj.set(size_atom, JSValue::new_int((cur_size + 1) as i64));
                        }
                    }
                }
            }
        }
    }

    let size_val = obj.get(size_atom).unwrap_or(JSValue::new_int(0));
    let size_prop_atom = ctx.intern("size");
    obj.set(size_prop_atom, size_val);

    JSValue::new_object(Box::into_raw(Box::new(obj)) as usize)
}

fn sync_map_size(ctx: &mut JSContext, obj: &mut JSObject) {
    let size_atom = ctx.intern("__map_size__");
    let size_prop_atom = ctx.intern("size");
    let size_val = obj.get(size_atom).unwrap_or(JSValue::new_int(0));
    obj.set(size_prop_atom, size_val);
}

fn map_set(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 {
        return if args.is_empty() {
            JSValue::undefined()
        } else {
            args[0]
        };
    }
    let this = args[0];
    if !this.is_object() {
        return this;
    }
    let key = args[1];
    let value = args[2];

    let obj = this.as_object_mut();

    let size_atom = ctx.intern("__map_size__");

    if let Some(idx) = map_find(ctx, obj, &key) {
        let mv_atom = map_val_atom(ctx, idx);
        obj.set(mv_atom, value);
    } else {
        let cur_size = map_size(ctx, obj);
        let mk_atom = map_key_atom(ctx, cur_size);
        let mv_atom = map_val_atom(ctx, cur_size);
        obj.set(mk_atom, key);
        obj.set(mv_atom, value);
        obj.set(size_atom, JSValue::new_int((cur_size + 1) as i64));
    }

    sync_map_size(ctx, obj);
    this
}

fn map_get(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let key = args[1];
    let obj = this.as_object();

    if let Some(idx) = map_find(ctx, obj, &key) {
        let mv_atom = map_val_atom(ctx, idx);
        obj.get(mv_atom).unwrap_or_else(JSValue::undefined)
    } else {
        JSValue::undefined()
    }
}

fn map_has(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::bool(false);
    }
    let key = args[1];
    let obj = this.as_object();
    JSValue::bool(map_find(ctx, obj, &key).is_some())
}

fn map_delete(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::bool(false);
    }
    let key = args[1];
    let obj = this.as_object_mut();

    let size = map_size(ctx, obj);
    let idx = match map_find(ctx, obj, &key) {
        Some(i) => i,
        None => return JSValue::bool(false),
    };

    for i in idx..(size - 1) {
        let mk_next = map_key_atom(ctx, i + 1);
        let mv_next = map_val_atom(ctx, i + 1);
        let mk_cur = map_key_atom(ctx, i);
        let mv_cur = map_val_atom(ctx, i);
        let k = obj.get(mk_next).unwrap_or_else(JSValue::undefined);
        let v = obj.get(mv_next).unwrap_or_else(JSValue::undefined);
        obj.set(mk_cur, k);
        obj.set(mv_cur, v);
    }

    let mk_last = map_key_atom(ctx, size - 1);
    let mv_last = map_val_atom(ctx, size - 1);
    obj.delete(mk_last);
    obj.delete(mv_last);

    let size_atom = ctx.intern("__map_size__");
    obj.set(size_atom, JSValue::new_int((size - 1) as i64));
    sync_map_size(ctx, obj);
    JSValue::bool(true)
}

fn map_clear(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let obj = this.as_object_mut();

    let size = map_size(ctx, obj);
    for i in 0..size {
        let mk = map_key_atom(ctx, i);
        let mv = map_val_atom(ctx, i);
        obj.delete(mk);
        obj.delete(mv);
    }
    let size_atom = ctx.intern("__map_size__");
    obj.set(size_atom, JSValue::new_int(0));
    sync_map_size(ctx, obj);
    JSValue::undefined()
}

fn map_for_each(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let this = args[0];
    let callback = args[1];
    if !this.is_object() || !callback.is_function() {
        return JSValue::undefined();
    }

    let obj = this.as_object();
    let size = map_size(ctx, obj);

    for i in 0..size {
        let mk_atom = map_key_atom(ctx, i);
        let mv_atom = map_val_atom(ctx, i);
        let k = obj.get(mk_atom).unwrap_or_else(JSValue::undefined);
        let v = obj.get(mv_atom).unwrap_or_else(JSValue::undefined);
        let _ = call_callback(ctx, callback, &[v, k, this]);
    }

    JSValue::undefined()
}

fn map_keys(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return make_array(ctx, vec![]);
    }
    let this = args[0];
    if !this.is_object() {
        return make_array(ctx, vec![]);
    }
    let obj = this.as_object();
    let size = map_size(ctx, obj);

    let mut keys = Vec::with_capacity(size);
    for i in 0..size {
        let mk_atom = map_key_atom(ctx, i);
        keys.push(obj.get(mk_atom).unwrap_or_else(JSValue::undefined));
    }
    make_array(ctx, keys)
}

fn map_values(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return make_array(ctx, vec![]);
    }
    let this = args[0];
    if !this.is_object() {
        return make_array(ctx, vec![]);
    }
    let obj = this.as_object();
    let size = map_size(ctx, obj);

    let mut vals = Vec::with_capacity(size);
    for i in 0..size {
        let mv_atom = map_val_atom(ctx, i);
        vals.push(obj.get(mv_atom).unwrap_or_else(JSValue::undefined));
    }
    make_array(ctx, vals)
}

fn map_entries(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return make_array(ctx, vec![]);
    }
    let this = args[0];
    if !this.is_object() {
        return make_array(ctx, vec![]);
    }
    let obj = this.as_object();
    let size = map_size(ctx, obj);

    let mut entries = Vec::with_capacity(size);
    for i in 0..size {
        let mk_atom = map_key_atom(ctx, i);
        let mv_atom = map_val_atom(ctx, i);
        let k = obj.get(mk_atom).unwrap_or_else(JSValue::undefined);
        let v = obj.get(mv_atom).unwrap_or_else(JSValue::undefined);
        let pair = make_array(ctx, vec![k, v]);
        entries.push(pair);
    }
    make_array(ctx, entries)
}

fn map_symbol_iterator(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    map_entries(ctx, args)
}

fn set_symbol_iterator(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    set_values(ctx, args)
}

fn weakmap_constructor(ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    let mut obj = JSObject::new();
    if let Some(proto_ptr) = ctx.get_weakmap_prototype() {
        obj.prototype = Some(proto_ptr);
    }
    let size_atom = ctx.intern("__weakmap_size__");
    obj.set(size_atom, JSValue::new_int(0));
    let ptr = Box::into_raw(Box::new(obj)) as usize;
    JSValue::new_object(ptr)
}

fn weakmap_size(ctx: &mut JSContext, obj: &JSObject) -> usize {
    let size_atom = ctx.intern("__weakmap_size__");
    obj.get(size_atom)
        .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
        .unwrap_or(0)
}

fn weakmap_key_atom(ctx: &mut JSContext, i: usize) -> crate::runtime::atom::Atom {
    ctx.intern(&format!("__wmk_{}__", i))
}

fn weakmap_val_atom(ctx: &mut JSContext, i: usize) -> crate::runtime::atom::Atom {
    ctx.intern(&format!("__wmv_{}__", i))
}

fn weakmap_find(ctx: &mut JSContext, obj: &JSObject, search_key: &JSValue) -> Option<usize> {
    let size = weakmap_size(ctx, obj);
    for i in 0..size {
        let k_atom = weakmap_key_atom(ctx, i);
        if let Some(k) = obj.get(k_atom) {
            if k.strict_eq(search_key) {
                return Some(i);
            }
        }
    }
    None
}

fn weakmap_set(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 {
        return JSValue::undefined();
    }
    let this = &args[0];
    let key = &args[1];
    let value = &args[2];

    if !key.is_object() && !key.is_symbol() {
        return JSValue::undefined();
    }

    let obj = this.as_object_mut();

    if let Some(idx) = weakmap_find(ctx, obj, key) {
        let v_atom = weakmap_val_atom(ctx, idx);
        obj.set(v_atom, value.clone());
    } else {
        let size_atom = ctx.intern("__weakmap_size__");
        let size = weakmap_size(ctx, obj);
        let k_atom = weakmap_key_atom(ctx, size);
        let v_atom = weakmap_val_atom(ctx, size);
        obj.set(k_atom, key.clone());
        obj.set(v_atom, value.clone());
        obj.set(size_atom, JSValue::new_int((size + 1) as i64));
    }
    this.clone()
}

fn weakmap_get(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let this = &args[0];
    let key = &args[1];

    if !this.is_object() {
        return JSValue::undefined();
    }
    let obj = this.as_object();

    if let Some(idx) = weakmap_find(ctx, obj, key) {
        let v_atom = weakmap_val_atom(ctx, idx);
        obj.get(v_atom).unwrap_or_else(JSValue::undefined)
    } else {
        JSValue::undefined()
    }
}

fn weakmap_has(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = &args[0];
    let key = &args[1];

    if !this.is_object() {
        return JSValue::bool(false);
    }
    let obj = this.as_object();

    JSValue::bool(weakmap_find(ctx, obj, key).is_some())
}

fn weakmap_delete(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = &args[0];
    let key = &args[1];

    if !this.is_object() {
        return JSValue::bool(false);
    }
    let obj = this.as_object_mut();

    if let Some(idx) = weakmap_find(ctx, obj, key) {
        let k_atom = weakmap_key_atom(ctx, idx);
        let v_atom = weakmap_val_atom(ctx, idx);
        obj.delete(k_atom);
        obj.delete(v_atom);
        JSValue::bool(true)
    } else {
        JSValue::bool(false)
    }
}

fn weakset_constructor(ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    let mut obj = JSObject::new();
    if let Some(proto_ptr) = ctx.get_weakset_prototype() {
        obj.prototype = Some(proto_ptr);
    }
    let size_atom = ctx.intern("__weakset_size__");
    obj.set(size_atom, JSValue::new_int(0));
    let ptr = Box::into_raw(Box::new(obj)) as usize;
    JSValue::new_object(ptr)
}

fn weakset_size(ctx: &mut JSContext, obj: &JSObject) -> usize {
    let size_atom = ctx.intern("__weakset_size__");
    obj.get(size_atom)
        .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
        .unwrap_or(0)
}

fn weakset_val_atom(ctx: &mut JSContext, i: usize) -> crate::runtime::atom::Atom {
    ctx.intern(&format!("__wsv_{}__", i))
}

fn weakset_find(ctx: &mut JSContext, obj: &JSObject, search: &JSValue) -> Option<usize> {
    let size = weakset_size(ctx, obj);
    for i in 0..size {
        let v_atom = weakset_val_atom(ctx, i);
        if let Some(v) = obj.get(v_atom) {
            if v.strict_eq(search) {
                return Some(i);
            }
        }
    }
    None
}

fn weakset_add(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let this = &args[0];
    let value = &args[1];

    if !value.is_object() && !value.is_symbol() {
        return JSValue::undefined();
    }

    let obj = this.as_object_mut();

    if weakset_find(ctx, obj, value).is_none() {
        let size_atom = ctx.intern("__weakset_size__");
        let size = weakset_size(ctx, obj);
        let v_atom = weakset_val_atom(ctx, size);
        obj.set(v_atom, value.clone());
        obj.set(size_atom, JSValue::new_int((size + 1) as i64));
    }
    this.clone()
}

fn weakset_has(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = &args[0];
    let value = &args[1];

    if !this.is_object() {
        return JSValue::bool(false);
    }
    let obj = this.as_object();

    JSValue::bool(weakset_find(ctx, obj, value).is_some())
}

fn weakset_delete(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = &args[0];
    let value = &args[1];

    if !this.is_object() {
        return JSValue::bool(false);
    }
    let obj = this.as_object_mut();

    if let Some(idx) = weakset_find(ctx, obj, value) {
        let v_atom = weakset_val_atom(ctx, idx);
        obj.delete(v_atom);
        JSValue::bool(true)
    } else {
        JSValue::bool(false)
    }
}

pub fn set_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut obj = JSObject::new();

    if let Some(proto_ptr) = ctx.get_set_prototype() {
        obj.prototype = Some(proto_ptr);
    }

    let size_atom = ctx.intern("__set_size__");
    obj.set(size_atom, JSValue::new_int(0));

    if let Some(iterable) = args.first() {
        if iterable.is_object() {
            let iter_obj = iterable.as_object();
            let len_atom = ctx.common_atoms.length;
            let len = iter_obj
                .get(len_atom)
                .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
                .unwrap_or(0);

            for i in 0..len {
                let idx_atom = ctx.intern(&i.to_string());
                if let Some(val) = iter_obj.get(idx_atom) {
                    let cur_size = set_size(ctx, &obj);
                    if set_find(ctx, &obj, &val).is_none() {
                        let sv_atom = set_val_atom(ctx, cur_size);
                        obj.set(sv_atom, val);
                        obj.set(size_atom, JSValue::new_int((cur_size + 1) as i64));
                    }
                }
            }
        }
    }

    let size_val = obj.get(size_atom).unwrap_or(JSValue::new_int(0));
    let size_prop_atom = ctx.intern("size");
    obj.set(size_prop_atom, size_val);

    JSValue::new_object(Box::into_raw(Box::new(obj)) as usize)
}

fn sync_set_size(ctx: &mut JSContext, obj: &mut JSObject) {
    let size_atom = ctx.intern("__set_size__");
    let size_prop_atom = ctx.intern("size");
    let size_val = obj.get(size_atom).unwrap_or(JSValue::new_int(0));
    obj.set(size_prop_atom, size_val);
}

fn set_add(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return if args.is_empty() {
            JSValue::undefined()
        } else {
            args[0]
        };
    }
    let this = args[0];
    if !this.is_object() {
        return this;
    }
    let value = args[1];

    let obj = this.as_object_mut();

    if set_find(ctx, obj, &value).is_none() {
        let size_atom = ctx.intern("__set_size__");
        let cur_size = set_size(ctx, obj);
        let sv_atom = set_val_atom(ctx, cur_size);
        obj.set(sv_atom, value);
        obj.set(size_atom, JSValue::new_int((cur_size + 1) as i64));
        sync_set_size(ctx, obj);
    }

    this
}

fn set_has(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::bool(false);
    }
    let value = args[1];
    let obj = this.as_object();
    JSValue::bool(set_find(ctx, obj, &value).is_some())
}

fn set_delete(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::bool(false);
    }
    let value = args[1];
    let obj = this.as_object_mut();

    let size = set_size(ctx, obj);
    let idx = match set_find(ctx, obj, &value) {
        Some(i) => i,
        None => return JSValue::bool(false),
    };

    for i in idx..(size - 1) {
        let sv_next = set_val_atom(ctx, i + 1);
        let sv_cur = set_val_atom(ctx, i);
        let v = obj.get(sv_next).unwrap_or_else(JSValue::undefined);
        obj.set(sv_cur, v);
    }

    let sv_last = set_val_atom(ctx, size - 1);
    obj.delete(sv_last);

    let size_atom = ctx.intern("__set_size__");
    obj.set(size_atom, JSValue::new_int((size - 1) as i64));
    sync_set_size(ctx, obj);
    JSValue::bool(true)
}

fn set_clear(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }
    let obj = this.as_object_mut();

    let size = set_size(ctx, obj);
    for i in 0..size {
        let sv = set_val_atom(ctx, i);
        obj.delete(sv);
    }
    let size_atom = ctx.intern("__set_size__");
    obj.set(size_atom, JSValue::new_int(0));
    sync_set_size(ctx, obj);
    JSValue::undefined()
}

fn set_for_each(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let this = args[0];
    let callback = args[1];
    if !this.is_object() || !callback.is_function() {
        return JSValue::undefined();
    }

    let obj = this.as_object();
    let size = set_size(ctx, obj);

    for i in 0..size {
        let sv_atom = set_val_atom(ctx, i);
        let v = obj.get(sv_atom).unwrap_or_else(JSValue::undefined);

        let _ = call_callback(ctx, callback, &[v, v, this]);
    }

    JSValue::undefined()
}

fn set_values(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return make_array(ctx, vec![]);
    }
    let this = args[0];
    if !this.is_object() {
        return make_array(ctx, vec![]);
    }
    let obj = this.as_object();
    let size = set_size(ctx, obj);

    let mut vals = Vec::with_capacity(size);
    for i in 0..size {
        let sv_atom = set_val_atom(ctx, i);
        vals.push(obj.get(sv_atom).unwrap_or_else(JSValue::undefined));
    }
    make_array(ctx, vals)
}

fn set_keys(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    set_values(ctx, args)
}

fn set_entries(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return make_array(ctx, vec![]);
    }
    let this = args[0];
    if !this.is_object() {
        return make_array(ctx, vec![]);
    }
    let obj = this.as_object();
    let size = set_size(ctx, obj);

    let mut entries = Vec::with_capacity(size);
    for i in 0..size {
        let sv_atom = set_val_atom(ctx, i);
        let v = obj.get(sv_atom).unwrap_or_else(JSValue::undefined);
        let pair = make_array(ctx, vec![v, v]);
        entries.push(pair);
    }
    make_array(ctx, entries)
}

pub fn init_map_set(ctx: &mut JSContext) {
    let mut map_proto = JSObject::new();
    map_proto.set(ctx.intern("set"), create_builtin_function(ctx, "map_set"));
    map_proto.set(ctx.intern("get"), create_builtin_function(ctx, "map_get"));
    map_proto.set(ctx.intern("has"), create_builtin_function(ctx, "map_has"));
    map_proto.set(
        ctx.intern("delete"),
        create_builtin_function(ctx, "map_delete"),
    );
    map_proto.set(
        ctx.intern("clear"),
        create_builtin_function(ctx, "map_clear"),
    );
    map_proto.set(
        ctx.intern("forEach"),
        create_builtin_function(ctx, "map_forEach"),
    );
    map_proto.set(ctx.intern("keys"), create_builtin_function(ctx, "map_keys"));
    map_proto.set(
        ctx.intern("values"),
        create_builtin_function(ctx, "map_values"),
    );
    map_proto.set(
        ctx.intern("entries"),
        create_builtin_function(ctx, "map_entries"),
    );

    let sym_iter_atom = crate::builtins::symbol::get_symbol_iterator_atom(ctx);
    map_proto.set(
        sym_iter_atom,
        create_builtin_function(ctx, "map_symbol_iterator"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        map_proto.prototype = Some(obj_proto_ptr);
    }

    let map_proto_ptr = Box::into_raw(Box::new(map_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(map_proto_ptr);
    ctx.set_map_prototype(map_proto_ptr);

    let map_ctor = create_builtin_function(ctx, "map_constructor");
    let map_atom = ctx.intern("Map");
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(map_atom, map_ctor);
    }

    let mut set_proto = JSObject::new();
    set_proto.set(ctx.intern("add"), create_builtin_function(ctx, "set_add"));
    set_proto.set(ctx.intern("has"), create_builtin_function(ctx, "set_has"));
    set_proto.set(
        ctx.intern("delete"),
        create_builtin_function(ctx, "set_delete"),
    );
    set_proto.set(
        ctx.intern("clear"),
        create_builtin_function(ctx, "set_clear"),
    );
    set_proto.set(
        ctx.intern("forEach"),
        create_builtin_function(ctx, "set_forEach"),
    );
    set_proto.set(
        ctx.intern("values"),
        create_builtin_function(ctx, "set_values"),
    );
    set_proto.set(ctx.intern("keys"), create_builtin_function(ctx, "set_keys"));
    set_proto.set(
        ctx.intern("entries"),
        create_builtin_function(ctx, "set_entries"),
    );

    let sym_iter_atom = crate::builtins::symbol::get_symbol_iterator_atom(ctx);
    set_proto.set(
        sym_iter_atom,
        create_builtin_function(ctx, "set_symbol_iterator"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        set_proto.prototype = Some(obj_proto_ptr);
    }

    let set_proto_ptr = Box::into_raw(Box::new(set_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(set_proto_ptr);
    ctx.set_set_prototype(set_proto_ptr);

    let set_ctor = create_builtin_function(ctx, "set_constructor");
    let set_atom = ctx.intern("Set");
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(set_atom, set_ctor);
    }

    let mut weakmap_proto = JSObject::new();
    weakmap_proto.set(
        ctx.intern("set"),
        create_builtin_function(ctx, "weakmap_set"),
    );
    weakmap_proto.set(
        ctx.intern("get"),
        create_builtin_function(ctx, "weakmap_get"),
    );
    weakmap_proto.set(
        ctx.intern("has"),
        create_builtin_function(ctx, "weakmap_has"),
    );
    weakmap_proto.set(
        ctx.intern("delete"),
        create_builtin_function(ctx, "weakmap_delete"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        weakmap_proto.prototype = Some(obj_proto_ptr);
    }

    let weakmap_proto_ptr = Box::into_raw(Box::new(weakmap_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(weakmap_proto_ptr);
    ctx.set_weakmap_prototype(weakmap_proto_ptr);

    let weakmap_ctor = create_builtin_function(ctx, "weakmap_constructor");
    let weakmap_atom = ctx.intern("WeakMap");
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(weakmap_atom, weakmap_ctor);
    }

    let mut weakset_proto = JSObject::new();
    weakset_proto.set(
        ctx.intern("add"),
        create_builtin_function(ctx, "weakset_add"),
    );
    weakset_proto.set(
        ctx.intern("has"),
        create_builtin_function(ctx, "weakset_has"),
    );
    weakset_proto.set(
        ctx.intern("delete"),
        create_builtin_function(ctx, "weakset_delete"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        weakset_proto.prototype = Some(obj_proto_ptr);
    }

    let weakset_proto_ptr = Box::into_raw(Box::new(weakset_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(weakset_proto_ptr);
    ctx.set_weakset_prototype(weakset_proto_ptr);

    let weakset_ctor = create_builtin_function(ctx, "weakset_constructor");
    let weakset_atom = ctx.intern("WeakSet");
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(weakset_atom, weakset_ctor);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "map_constructor",
        HostFunction::new("Map", 0, map_constructor),
    );
    ctx.register_builtin("map_set", HostFunction::new("set", 2, map_set));
    ctx.register_builtin("map_get", HostFunction::new("get", 1, map_get));
    ctx.register_builtin("map_has", HostFunction::new("has", 1, map_has));
    ctx.register_builtin("map_delete", HostFunction::new("delete", 1, map_delete));
    ctx.register_builtin("map_clear", HostFunction::new("clear", 0, map_clear));
    ctx.register_builtin("map_forEach", HostFunction::new("forEach", 1, map_for_each));
    ctx.register_builtin("map_keys", HostFunction::new("keys", 0, map_keys));
    ctx.register_builtin("map_values", HostFunction::new("values", 0, map_values));
    ctx.register_builtin("map_entries", HostFunction::new("entries", 0, map_entries));
    ctx.register_builtin(
        "map_symbol_iterator",
        HostFunction::new("[Symbol.iterator]", 0, map_symbol_iterator),
    );

    ctx.register_builtin(
        "set_constructor",
        HostFunction::new("Set", 0, set_constructor),
    );
    ctx.register_builtin("set_add", HostFunction::new("add", 1, set_add));
    ctx.register_builtin("set_has", HostFunction::new("has", 1, set_has));
    ctx.register_builtin("set_delete", HostFunction::new("delete", 1, set_delete));
    ctx.register_builtin("set_clear", HostFunction::new("clear", 0, set_clear));
    ctx.register_builtin("set_forEach", HostFunction::new("forEach", 1, set_for_each));
    ctx.register_builtin("set_values", HostFunction::new("values", 0, set_values));
    ctx.register_builtin("set_keys", HostFunction::new("keys", 0, set_keys));
    ctx.register_builtin("set_entries", HostFunction::new("entries", 0, set_entries));
    ctx.register_builtin(
        "set_symbol_iterator",
        HostFunction::new("[Symbol.iterator]", 0, set_symbol_iterator),
    );

    ctx.register_builtin(
        "weakmap_constructor",
        HostFunction::new("WeakMap", 0, weakmap_constructor),
    );
    ctx.register_builtin("weakmap_set", HostFunction::new("set", 2, weakmap_set));
    ctx.register_builtin("weakmap_get", HostFunction::new("get", 1, weakmap_get));
    ctx.register_builtin("weakmap_has", HostFunction::new("has", 1, weakmap_has));
    ctx.register_builtin(
        "weakmap_delete",
        HostFunction::new("delete", 1, weakmap_delete),
    );

    ctx.register_builtin(
        "weakset_constructor",
        HostFunction::new("WeakSet", 0, weakset_constructor),
    );
    ctx.register_builtin("weakset_add", HostFunction::new("add", 1, weakset_add));
    ctx.register_builtin("weakset_has", HostFunction::new("has", 1, weakset_has));
    ctx.register_builtin(
        "weakset_delete",
        HostFunction::new("delete", 1, weakset_delete),
    );
}
