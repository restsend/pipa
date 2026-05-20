use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub trait ToJS {
    fn to_js(&self, ctx: &mut JSContext) -> JSValue;
}

pub trait FromJS: Sized {
    fn from_js(value: &JSValue, ctx: &JSContext) -> Option<Self>;
}

impl ToJS for () {
    fn to_js(&self, _ctx: &mut JSContext) -> JSValue {
        JSValue::undefined()
    }
}

impl ToJS for bool {
    fn to_js(&self, _ctx: &mut JSContext) -> JSValue {
        JSValue::bool(*self)
    }
}

impl ToJS for i32 {
    fn to_js(&self, _ctx: &mut JSContext) -> JSValue {
        JSValue::new_int(*self as i64)
    }
}

impl ToJS for i64 {
    fn to_js(&self, _ctx: &mut JSContext) -> JSValue {
        JSValue::new_int(*self)
    }
}

impl ToJS for f64 {
    fn to_js(&self, _ctx: &mut JSContext) -> JSValue {
        JSValue::new_float(*self)
    }
}

impl ToJS for String {
    fn to_js(&self, ctx: &mut JSContext) -> JSValue {
        JSValue::new_string(ctx.intern(self))
    }
}

impl ToJS for &str {
    fn to_js(&self, ctx: &mut JSContext) -> JSValue {
        JSValue::new_string(ctx.intern(self))
    }
}

impl<T: ToJS> ToJS for Option<T> {
    fn to_js(&self, ctx: &mut JSContext) -> JSValue {
        match self {
            Some(v) => v.to_js(ctx),
            None => JSValue::null(),
        }
    }
}

impl FromJS for () {
    fn from_js(_value: &JSValue, _ctx: &JSContext) -> Option<Self> {
        Some(())
    }
}

impl FromJS for bool {
    fn from_js(value: &JSValue, _ctx: &JSContext) -> Option<Self> {
        if value.is_bool() {
            Some(value.get_bool())
        } else {
            None
        }
    }
}

impl FromJS for i32 {
    fn from_js(value: &JSValue, _ctx: &JSContext) -> Option<Self> {
        if value.is_int() {
            Some(value.get_int() as i32)
        } else {
            None
        }
    }
}

impl FromJS for i64 {
    fn from_js(value: &JSValue, _ctx: &JSContext) -> Option<Self> {
        if value.is_int() {
            Some(value.get_int())
        } else {
            None
        }
    }
}

impl FromJS for f64 {
    fn from_js(value: &JSValue, _ctx: &JSContext) -> Option<Self> {
        if value.is_int() {
            Some(value.get_int() as f64)
        } else if value.is_float() {
            Some(value.get_float())
        } else {
            None
        }
    }
}

impl FromJS for String {
    fn from_js(value: &JSValue, ctx: &JSContext) -> Option<Self> {
        if value.is_string() {
            let atom = value.get_atom();
            Some(ctx.get_atom_str(atom).to_string())
        } else {
            None
        }
    }
}
