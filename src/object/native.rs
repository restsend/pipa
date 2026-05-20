use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub type NativeFunc = fn(&mut JSContext, &[JSValue]) -> JSValue;

pub struct NativeFunction {
    pub func: NativeFunc,
    pub name: &'static str,
    pub arity: u32,
}

impl NativeFunction {
    pub fn new(name: &'static str, arity: u32, func: NativeFunc) -> Self {
        NativeFunction { name, arity, func }
    }

    pub fn call(&self, ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
        (self.func)(ctx, args)
    }
}
