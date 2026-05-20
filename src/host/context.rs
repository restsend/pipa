use crate::object::native::NativeFunc;
use crate::runtime::atom::Atom;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub trait HostContext {
    fn register_host_object<T: HostObject>(&mut self, name: &str, obj: T);
    fn register_native_func(&mut self, name: &str, func: NativeFunc);
    fn create_dom_node(&mut self, node: DomNodeHandle) -> JSValue;
}

pub struct DomNodeHandle(u64);

impl DomNodeHandle {
    pub fn new(id: u64) -> Self {
        DomNodeHandle(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

pub trait HostObject: 'static {
    fn get(&self, ctx: &JSContext, prop: Atom) -> JSValue;
    fn set(&mut self, ctx: &mut JSContext, prop: Atom, value: JSValue) -> Result<(), ()>;
    fn call(&mut self, ctx: &mut JSContext, args: &[JSValue]) -> JSValue;
    fn construct(&mut self, ctx: &mut JSContext, args: &[JSValue]) -> JSValue;
}
