use super::context::JSContext;
use std::any::Any;

pub trait MacroTaskExtension {
    fn tick(&mut self, ctx: &mut JSContext) -> Result<bool, String>;
    fn has_pending(&self) -> bool;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
