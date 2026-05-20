use super::context::JSContext;
pub trait MacroTaskExtension {
    fn tick(&mut self, ctx: &mut JSContext) -> Result<bool, String>;
    fn has_pending(&self) -> bool;
}
