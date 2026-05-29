use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::atom::Atom;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn create_symbol(ctx: &mut JSContext, description: Option<Atom>) -> JSValue {
    let desc = description.unwrap_or(ctx.intern(""));
    ctx.mark_symbol_atom(desc);
    let id = ctx.runtime_mut().next_symbol_id();
    JSValue::new_symbol_with_id(desc, id)
}

fn symbol_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let desc = if args.is_empty() {
        None
    } else {
        let desc_val = args[0];
        if desc_val.is_string() {
            Some(desc_val.get_atom())
        } else if desc_val.is_undefined() {
            None
        } else {
            let s = if desc_val.is_int() {
                desc_val.get_int().to_string()
            } else if desc_val.is_float() {
                desc_val.get_float().to_string()
            } else if desc_val.is_bool() {
                desc_val.get_bool().to_string()
            } else {
                String::new()
            };
            Some(ctx.intern(&s))
        }
    };
    create_symbol(ctx, desc)
}

fn symbol_for(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let key_val = args[0];
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else {
        let s = if key_val.is_int() {
            key_val.get_int().to_string()
        } else if key_val.is_float() {
            key_val.get_float().to_string()
        } else {
            String::new()
        };
        ctx.intern(&s)
    };

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        let reg_atom = ctx.intern("__symbol_registry__");
        if let Some(reg) = global_obj.get(reg_atom) {
            if reg.is_object() {
                let reg_obj = reg.as_object_mut();
                let len_atom = ctx.common_atoms.length;
                let len = reg_obj
                    .get(len_atom)
                    .map(|v| v.get_int() as usize)
                    .unwrap_or(0);
                for i in 0..len {
                    let idx_atom = ctx.intern(&i.to_string());
                    if let Some(entry) = reg_obj.get(idx_atom) {
                        if entry.is_object() {
                            let entry_obj = entry.as_object_mut();
                            let desc_atom = ctx.intern("description");
                            if let Some(desc) = entry_obj.get(desc_atom) {
                                if desc.is_string() && desc.get_atom() == key {
                                    let sym_atom = ctx.intern("symbol");
                                    if let Some(sym) = entry_obj.get(sym_atom) {
                                        return sym;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let new_sym = create_symbol(ctx, Some(key));

        let reg = if let Some(r) = global_obj.get(reg_atom) {
            if r.is_object() {
                unsafe { JSValue::object_from_ptr_mut(r.get_ptr()) }
            } else {
                let mut new_reg = JSObject::new_array();
                new_reg.set(ctx.common_atoms.length, JSValue::new_int(0));
                let reg_ptr = Box::into_raw(Box::new(new_reg)) as usize;
                let val = JSValue::new_object(reg_ptr);
                global_obj.set(reg_atom, val);

                unsafe { JSValue::object_from_ptr_mut(reg_ptr) }
            }
        } else {
            let mut new_reg = JSObject::new_array();
            new_reg.set(ctx.common_atoms.length, JSValue::new_int(0));
            let reg_ptr = Box::into_raw(Box::new(new_reg)) as usize;
            let val = JSValue::new_object(reg_ptr);
            global_obj.set(reg_atom, val);

            unsafe { JSValue::object_from_ptr_mut(reg_ptr) }
        };

        let len_atom = ctx.common_atoms.length;
        let len = reg.get(len_atom).map(|v| v.get_int() as usize).unwrap_or(0);
        let mut entry = JSObject::new();
        entry.set(ctx.intern("description"), JSValue::new_string(key));
        entry.set(ctx.intern("symbol"), new_sym);
        let entry_ptr = Box::into_raw(Box::new(entry)) as usize;
        let entry_val = JSValue::new_object(entry_ptr);
        let idx_atom = ctx.intern(&len.to_string());
        reg.set(idx_atom, entry_val);
        reg.set(len_atom, JSValue::new_int((len + 1) as i64));

        return new_sym;
    }

    create_symbol(ctx, Some(key))
}

fn symbol_key_for(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let sym_val = args[0];
    if !sym_val.is_symbol() {
        return JSValue::undefined();
    }

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object();
        let reg_atom = ctx.intern("__symbol_registry__");
        if let Some(reg) = global_obj.get(reg_atom) {
            if reg.is_object() {
                let reg_obj = reg.as_object();
                let len_atom = ctx.common_atoms.length;
                let len = reg_obj
                    .get(len_atom)
                    .map(|v| v.get_int() as usize)
                    .unwrap_or(0);
                for i in 0..len {
                    let idx_atom = ctx.intern(&i.to_string());
                    if let Some(entry) = reg_obj.get(idx_atom) {
                        if entry.is_object() {
                            let entry_obj = entry.as_object();
                            let sym_atom = ctx.intern("symbol");
                            if let Some(sym) = entry_obj.get(sym_atom) {
                                if sym.strict_eq(&sym_val) {
                                    let desc_atom = ctx.intern("description");
                                    if let Some(desc) = entry_obj.get(desc_atom) {
                                        return desc;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    JSValue::undefined()
}

fn symbol_value_of(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    if this_val.is_symbol() {
        *this_val
    } else {
        JSValue::undefined()
    }
}

fn symbol_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern("Symbol()"));
    }
    let this_val = &args[0];
    if this_val.is_symbol() {
        let desc = ctx.get_atom_str(this_val.get_atom());
        if desc.is_empty() {
            JSValue::new_string(ctx.intern("Symbol()"))
        } else {
            JSValue::new_string(ctx.intern(&format!("Symbol({})", desc)))
        }
    } else {
        JSValue::new_string(ctx.intern("Symbol()"))
    }
}

const SYMBOL_ITERATOR_DESC: &str = "Symbol.iterator";
const SYMBOL_TO_STRING_TAG_DESC: &str = "Symbol.toStringTag";
const SYMBOL_SPECIES_DESC: &str = "Symbol.species";
const SYMBOL_TO_PRIMITIVE_DESC: &str = "Symbol.toPrimitive";
const SYMBOL_IS_CONCAT_SPREADABLE_DESC: &str = "Symbol.isConcatSpreadable";
const SYMBOL_MATCH_DESC: &str = "Symbol.match";
const SYMBOL_REPLACE_DESC: &str = "Symbol.replace";
const SYMBOL_SEARCH_DESC: &str = "Symbol.search";
const SYMBOL_SPLIT_DESC: &str = "Symbol.split";
const SYMBOL_UNSCOPABLES_DESC: &str = "Symbol.unscopables";
const SYMBOL_HAS_INSTANCE_DESC: &str = "Symbol.hasInstance";
const SYMBOL_ASYNC_ITERATOR_DESC: &str = "Symbol.asyncIterator";

pub fn get_or_create_well_known_symbol(ctx: &mut JSContext, description: &str) -> JSValue {
    let desc_atom = ctx.intern(description);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        if let Some(sym) = global_obj.get(desc_atom) {
            return sym;
        }
        let sym = create_symbol(ctx, Some(desc_atom));
        global_obj.set(desc_atom, sym);
        return sym;
    }
    create_symbol(ctx, Some(desc_atom))
}

pub fn init_symbol(ctx: &mut JSContext) {
    let symbol_atom = ctx.intern("Symbol");
    let global = ctx.global();
    if global.is_object() {
        if global.as_object().get(symbol_atom).is_some() {
            return;
        }
    }
    let mut symbol_ctor = JSFunction::new_builtin(ctx.intern("Symbol"), 1);
    symbol_ctor.set_builtin_marker(ctx, "symbol_constructor");

    let for_func = create_builtin_function(ctx, "symbol_for");
    let key_for_func = create_builtin_function(ctx, "symbol_keyFor");

    symbol_ctor.base.define_property(
        ctx.intern("for"),
        crate::object::object::PropertyDescriptor {
            value: Some(for_func),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
    symbol_ctor.base.define_property(
        ctx.intern("keyFor"),
        crate::object::object::PropertyDescriptor {
            value: Some(key_for_func),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );

    let symbol_iter = get_or_create_well_known_symbol(ctx, SYMBOL_ITERATOR_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("iterator"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_iter),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_to_string_tag = get_or_create_well_known_symbol(ctx, SYMBOL_TO_STRING_TAG_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("toStringTag"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_to_string_tag),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_species = get_or_create_well_known_symbol(ctx, SYMBOL_SPECIES_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("species"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_species),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_to_primitive = get_or_create_well_known_symbol(ctx, SYMBOL_TO_PRIMITIVE_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("toPrimitive"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_to_primitive),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_is_concat_spreadable =
        get_or_create_well_known_symbol(ctx, SYMBOL_IS_CONCAT_SPREADABLE_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("isConcatSpreadable"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_is_concat_spreadable),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_match = get_or_create_well_known_symbol(ctx, SYMBOL_MATCH_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("match"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_match),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_replace = get_or_create_well_known_symbol(ctx, SYMBOL_REPLACE_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("replace"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_replace),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_search = get_or_create_well_known_symbol(ctx, SYMBOL_SEARCH_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("search"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_search),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_split = get_or_create_well_known_symbol(ctx, SYMBOL_SPLIT_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("split"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_split),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_unscopables = get_or_create_well_known_symbol(ctx, SYMBOL_UNSCOPABLES_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("unscopables"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_unscopables),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_has_instance = get_or_create_well_known_symbol(ctx, SYMBOL_HAS_INSTANCE_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("hasInstance"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_has_instance),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let symbol_async_iterator = get_or_create_well_known_symbol(ctx, SYMBOL_ASYNC_ITERATOR_DESC);
    symbol_ctor.base.define_property(
        ctx.intern("asyncIterator"),
        crate::object::object::PropertyDescriptor {
            value: Some(symbol_async_iterator),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );

    let mut sym_proto = JSObject::new();
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        sym_proto.prototype = Some(obj_proto_ptr);
    }
    sym_proto.set(
        ctx.intern("toString"),
        create_builtin_function(ctx, "symbol_toString"),
    );
    sym_proto.set(
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "symbol_valueOf"),
    );

    let proto_ptr = Box::into_raw(Box::new(sym_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    symbol_ctor.base.set(ctx.intern("prototype"), proto_value);
    ctx.set_symbol_prototype(proto_ptr);

    let symbol_ptr = Box::into_raw(Box::new(symbol_ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(symbol_ptr);
    let symbol_ctor_val = JSValue::new_function(symbol_ptr);

    let symbol_atom = ctx.intern("Symbol");
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, symbol_atom, symbol_ctor_val);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "symbol_constructor",
        HostFunction::new("Symbol", 1, symbol_constructor),
    );
    ctx.register_builtin("symbol_for", HostFunction::new("for", 1, symbol_for));
    ctx.register_builtin(
        "symbol_keyFor",
        HostFunction::new("keyFor", 1, symbol_key_for),
    );
    ctx.register_builtin(
        "symbol_valueOf",
        HostFunction::method("valueOf", 0, symbol_value_of),
    );
    ctx.register_builtin(
        "symbol_toString",
        HostFunction::method("toString", 0, symbol_to_string),
    );
}

pub fn get_symbol_iterator(ctx: &mut JSContext) -> JSValue {
    get_or_create_well_known_symbol(ctx, SYMBOL_ITERATOR_DESC)
}

pub fn get_symbol_iterator_atom(ctx: &mut JSContext) -> Atom {
    ctx.intern("Symbol.iterator")
}

pub fn get_symbol_split(ctx: &mut JSContext) -> JSValue {
    get_or_create_well_known_symbol(ctx, SYMBOL_SPLIT_DESC)
}

pub fn get_symbol_split_atom(ctx: &mut JSContext) -> Atom {
    ctx.intern("Symbol.split")
}

pub fn get_symbol_unscopables(ctx: &mut JSContext) -> JSValue {
    get_or_create_well_known_symbol(ctx, SYMBOL_UNSCOPABLES_DESC)
}

pub fn get_symbol_unscopables_atom(ctx: &mut JSContext) -> Atom {
    ctx.intern("Symbol.unscopables")
}

pub fn get_symbol_has_instance(ctx: &mut JSContext) -> JSValue {
    get_or_create_well_known_symbol(ctx, SYMBOL_HAS_INSTANCE_DESC)
}

pub fn get_symbol_has_instance_atom(ctx: &mut JSContext) -> Atom {
    ctx.intern("Symbol.hasInstance")
}

pub fn get_symbol_async_iterator(ctx: &mut JSContext) -> JSValue {
    get_or_create_well_known_symbol(ctx, SYMBOL_ASYNC_ITERATOR_DESC)
}

pub fn get_symbol_async_iterator_atom(ctx: &mut JSContext) -> Atom {
    ctx.intern("Symbol.asyncIterator")
}

pub fn get_symbol_to_string_tag_prop_key(ctx: &mut JSContext) -> Atom {
    let sym = get_or_create_well_known_symbol(ctx, SYMBOL_TO_STRING_TAG_DESC);
    crate::runtime::atom::Atom(0x40000000 | sym.get_symbol_id())
}

pub fn is_symbol(val: &JSValue) -> bool {
    val.is_symbol()
}
