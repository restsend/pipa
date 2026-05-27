use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub type HostFunc = fn(&mut JSContext, &[JSValue]) -> JSValue;

#[derive(Clone, Copy)]
pub struct HostFunction {
    pub func: HostFunc,
    pub name: &'static str,
    pub arity: u32,
    pub needs_this: bool,
    pub is_constructor: bool,
}

impl HostFunction {
    pub fn new(name: &'static str, arity: u32, func: HostFunc) -> Self {
        HostFunction {
            name,
            arity,
            func,
            needs_this: false,
            is_constructor: false,
        }
    }

    pub fn method(name: &'static str, arity: u32, func: HostFunc) -> Self {
        HostFunction {
            name,
            arity,
            func,
            needs_this: true,
            is_constructor: false,
        }
    }

    pub fn ctor(name: &'static str, arity: u32, func: HostFunc) -> Self {
        HostFunction {
            name,
            arity,
            func,
            needs_this: false,
            is_constructor: true,
        }
    }

    pub fn call(&self, ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
        (self.func)(ctx, args)
    }
}
