use std::cell::Cell;

use game_engine::{
    core::GameState,
    ui::{Button, EventHandleResult, UIComponent, UI, UIState},
};
use sdl2::{rect::Rect, pixels::Color};

#[derive(PartialEq, Eq)]
enum FocusState {
    Idle,
    Hovered,
    Pressed,
}

struct BasicButton {
    bound: Rect,
    focus_state: FocusState,
}

impl Default for BasicButton {
    fn default() -> Self {
        Self {
            // these bounds will never be used, but just in case they are placed off screen.
            // Rect::contains_point is inclusive
            bound: Rect::new(-1, -1, 0, 0),
            focus_state: FocusState::Idle,
        }
    }
}

impl BasicButton {
    const IDLE_BACKGROUND: Color = Color::RGBA(100, 100, 100, 30);
    const HOVERED_BACKGROUND: Color = Color::RGBA(100, 100, 100, 50);
    const PRESSED_BACKGROUND: Color = Color::RGBA(100, 100, 100, 80);
}

impl UIComponent for BasicButton {
    fn process(&mut self, ui_state: &UIState, e: &sdl2::event::Event) -> EventHandleResult {
        Button::process(self, ui_state, e)
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        // draw inner background first
        canvas.set_draw_color(match self.focus_state {
            FocusState::Idle => Self::IDLE_BACKGROUND,
            FocusState::Hovered => Self::HOVERED_BACKGROUND,
            FocusState::Pressed => Self::PRESSED_BACKGROUND,
        });
        canvas.fill_rect(self.bound).unwrap();
    }

    fn resize(&mut self, window_size: (u32, u32)) {
        self.bound = Rect::new(10, 10, window_size.0 / 2, window_size.1 / 2);
    }
}

impl Button for BasicButton {
    fn bounds(&self) -> Rect {
        self.bound
    }

    fn moved_out(&mut self) {
        self.focus_state = FocusState::Idle;
    }
    
    fn moved_in(&mut self) {
        self.focus_state = FocusState::Hovered;
    }


    fn pressed(&mut self) {
        self.focus_state = FocusState::Pressed;
    }

    fn released(&mut self) -> EventHandleResult {
        println!("released");
        EventHandleResult::None
    }
}

fn main() -> Result<(), String> {
    let mut state = GameState::new("ui with layers of buttons", (800u32, 600u32), &[])?;

    let mut initial_buttons: Vec<Box<dyn UIComponent>> = Vec::new();
    initial_buttons.push(Box::new(BasicButton::default()));
    let mut ui = UI::new(state.window_size());
    ui.add(initial_buttons);

    let ui_cell = Cell::new(Option::Some(ui));

    state.run(
        |_, event| {
            match event {
                sdl2::event::Event::Quit { .. } => return Ok(false),
                _ => {}
            }
            let mut ui = ui_cell.take().unwrap();
            ui.process(event);
            ui_cell.set(Some(ui));
            Ok(true)
        },
        |canvas| {
            let ui = ui_cell.take().unwrap();
            ui.render(canvas);
            ui_cell.set(Some(ui));
        },
    )?;
    Ok(())
}
