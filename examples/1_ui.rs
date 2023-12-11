use std::cell::Cell;

use game_engine::{core::GameState, ui::{UI, Rect, Button, UIComponent}};

struct BasicButton {
    bound: Rect,
}

impl BasicButton {
    const WIDTH_PORTION: f32 = 0.1;
    const HEIGHT_PORTION: f32 = 0.03;
}

impl UIComponent for BasicButton {
    fn process(&mut self, e: &sdl2::event::Event) -> game_engine::ui::EventHandleResult {
        todo!()
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        todo!()
    }

    fn resize(&mut self, window_size: (u32, u32)) {
        todo!()
    }
}

impl Button for BasicButton {
    fn bounds(&self) -> Rect {
        todo!()
    }

    fn hover(&mut self) {
        todo!()
    }

    fn pressed(&mut self) {
        todo!()
    }

    fn released(&mut self) -> game_engine::ui::EventHandleResult {
        todo!()
    }

}

fn main() -> Result<(), String> {
    let mut state = GameState::new("ui with layers of buttons", (800u32, 600u32), &[])?;
    let ui_cell = Cell::new(Option::Some(UI::new(state.window_size())));

    state.run(
        |_, event| {
            match event {
                sdl2::event::Event::Quit { .. } => return Ok(false),
                _ => {}
            }
            let mut ui = ui_cell.take();
            ui.process(event);
            ui_cell.set(ui);
            Ok(true)
        },
        |canvas| {
            let ui = ui_cell.take();
            ui.render(canvas);
            ui_cell.set(ui);
        },
    )?;
    Ok(())
}
