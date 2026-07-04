pub mod array;
pub mod base64;
pub mod bigint;
pub mod date;
pub mod error;
#[cfg(feature = "fetch")]
pub mod eventsource;
#[cfg(feature = "fetch")]
pub mod fetch;
pub mod function;
pub mod generator;
pub mod global;
pub mod intl;
pub mod json;
pub mod json_parser;
pub mod map_set;
pub mod math;
pub mod number;
pub mod object;
pub mod parse;
#[cfg(feature = "process")]
pub mod process;
pub mod promise;
pub mod regexp;
pub mod string;
pub mod symbol;
pub mod typedarray;
pub mod unicode_data;
pub mod url;
pub mod weakref;
#[cfg(feature = "fetch")]
pub mod websocket;

pub use global::init_globals;

use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn new_array(ctx: &mut JSContext) -> JSValue {
    let mut arr = JSObject::new_array();
    if let Some(proto) = ctx.get_array_prototype() {
        arr.set_prototype_raw(proto);
    }
    let ptr = Box::into_raw(Box::new(arr)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_array(ptr);
    JSValue::new_object(ptr)
}

pub fn new_plain_object(ctx: &mut JSContext) -> JSValue {
    let mut obj = JSObject::new();
    obj.prototype = ctx.get_object_prototype();
    let ptr = Box::into_raw(Box::new(obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}
