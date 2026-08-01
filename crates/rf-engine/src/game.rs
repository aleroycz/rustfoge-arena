use crate::EngineContext;

pub trait Game: 'static {
    fn init(&mut self, ctx: &mut EngineContext);

    fn fixed_update(&mut self, _ctx: &mut EngineContext) {}

    fn update(&mut self, _ctx: &mut EngineContext) {}
}
