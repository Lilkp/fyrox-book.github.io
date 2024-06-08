use fyrox::engine::GraphicsContext;
use fyrox::{
    core::{reflect::prelude::*, visitor::prelude::*},
    plugin::{Plugin, PluginContext},
};

// ANCHOR: graphics_context
#[derive(Visit, Reflect, Debug)]
struct Game {}

impl Plugin for Game {
    fn on_graphics_context_initialized(&mut self, context: PluginContext) {
        // At this stage it is safe to call `as_initialized_mut`, because graphics context is guaranteed
        // to be alive when this method is being called.
        let graphics_context = context.graphics_context.as_initialized_mut();

        graphics_context.window.set_title("My Cool Game");
    }

    fn on_graphics_context_destroyed(&mut self, context: PluginContext) {
        println!("Graphics context was destroyed.")
    }
    // ANCHOR_END: graphics_context

    // ANCHOR: update
    fn update(&mut self, context: &mut PluginContext) {
        if let GraphicsContext::Initialized(graphics_context) = context.graphics_context {
            graphics_context.window.set_title(&format!(
                "FPS: {}",
                graphics_context.renderer.get_statistics().frames_per_second
            ));
        }
    }
    // ANCHOR_END: update
}
