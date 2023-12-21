use sdl2::{pixels::Color, rect::Rect};

use super::{font_manager::FontManager, ui::{UIComponent, UIState, EventHandleResult}};

/// a tint over the entire screen
pub struct Tint {
    pub color: Color,
}

impl<'sdl> UIComponent<'sdl> for Tint {
    fn process(
        &mut self,
        _: &UIState,
        _: &sdl2::event::Event,
    ) -> EventHandleResult<'sdl> {
        EventHandleResult::None
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        canvas.set_draw_color(self.color);
        canvas
            .fill_rect(Rect::new(0, 0, u32::MAX, u32::MAX))
            .unwrap();
    }

    fn resize(
        &mut self,
        _: (u32, u32),
        _: &'sdl sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        _: &FontManager,
    ) {
    }
}
