use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub type HostFunc = fn(&mut JSContext, &[JSValue]) -> JSValue;

#[derive(Clone, Copy)]
pub struct HostFunction {
    pub func: HostFunc,
    pub name: &'static str,
    pub arity: u32,
}

impl HostFunction {
    pub fn new(name: &'static str, arity: u32, func: HostFunc) -> Self {
        HostFunction { name, arity, func }
    }

    pub fn call(&self, ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
        (self.func)(ctx, args)
    }
}
