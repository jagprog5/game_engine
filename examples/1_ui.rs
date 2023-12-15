use std::{
    cell::Cell,
    path::{Path, PathBuf},
};

use game_engine::{
    core::GameState,
    ui::{Button, EventHandleResult, UIComponent, UIState, UI, FontManager},
};
use sdl2::{
    pixels::Color,
    rect::Rect,
    render::{TextureCreator, WindowCanvas, Texture},
    video::WindowContext,
};

#[derive(PartialEq, Eq)]
// button internal state for rendering
enum FocusState {
    Idle,
    Hovered,
    Pressed,
}

#[derive(PartialEq, Eq)]
// indicates the look, position, and action for ExampleButton
enum ExampleButtonType {
    NewGame,
    Quit,
}

impl ExampleButtonType {
    pub fn text(&self) -> &'static str {
        match self {
            ExampleButtonType::NewGame => "New Game",
            ExampleButtonType::Quit => "Quit",
        }
    }
}

struct ExampleButton<'sdl> {
    // rendered text. swapped out per call to resize
    rendered_text: Option<Texture<'sdl>>,

    bound: Rect,
    focus_state: FocusState,

    which: ExampleButtonType,
}

impl<'sdl> ExampleButton<'sdl> {
    const IDLE_BACKGROUND: Color = Color::RGBA(100, 100, 100, 30);
    const HOVERED_BACKGROUND: Color = Color::RGBA(100, 100, 100, 50);
    const PRESSED_BACKGROUND: Color = Color::RGBA(100, 100, 100, 80);

    const BUTTON_HEIGHT: u16 = 20;
    const BORDER_WIDTH: u16 = 2;

    fn new(which: ExampleButtonType) -> Self {
        Self {
            rendered_text: None,
            // these bounds will never be used (replaced on resize when added to UI),
            // but just in case this places it off screen. Rect::contains_point is
            // inclusive
            bound: Rect::new(-1, -1, 0, 0),
            focus_state: FocusState::Idle,
            which,
        }
    }
}

impl<'sdl> UIComponent<'sdl> for ExampleButton<'sdl> {
    fn process(
        &mut self,
        ui_state: &UIState,
        e: &sdl2::event::Event,
    ) -> EventHandleResult<'sdl> {
        Button::<'sdl>::process(self, ui_state, e)
    }

    fn render(&self, canvas: &mut WindowCanvas) {
        // draw inner background first
        canvas.set_draw_color(match self.focus_state {
            FocusState::Idle => Self::IDLE_BACKGROUND,
            FocusState::Hovered => Self::HOVERED_BACKGROUND,
            FocusState::Pressed => Self::PRESSED_BACKGROUND,
        });
        // background
        canvas.fill_rect(self.bound).unwrap();
        // text on top
    }

    fn resize(
        &mut self,
        window_size: (u32, u32),
        ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext,
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_manager: &mut FontManager,
    ) {
        // load the font with a specified height
        let font = ttf_context
            .load_font(
                Path::new(&self.font_path),
                Self::BUTTON_HEIGHT - Self::BORDER_WIDTH * 2,
            )
            .unwrap();

        let surface = font
            .render(self.text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();

        let texture = texture_creator.create_texture_from_surface(&surface).unwrap();

        let text_width = texture.query().width;
        let width = text_width + Self::BORDER_WIDTH as u32 * 2;
        self.rendered_text = Some(texture);

        let height = Self::BUTTON_HEIGHT as u32;

        let x = (window_size.0 / 2 - width / 2) as i32;
        let y = (window_size.1 / 2 - height / 2) as i32;
        self.bound = Rect::new(x, y, width, height);
    }
}

impl<'sdl> Button<'sdl> for ExampleButton<'sdl> {
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

    fn released(&mut self) -> EventHandleResult<'sdl> {
        println!("released");
        EventHandleResult::Clear
    }
}

fn main() -> Result<(), String> {
    let mut state = GameState::new("ui with layers of buttons", (800u32, 600u32), &[])?;
    let texture_creator = state.canvas.texture_creator();
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut initial_buttons: Vec<Box<dyn UIComponent>> = Vec::new();

    let mut font_path_buf = PathBuf::new();
    font_path_buf.push(file!());
    font_path_buf.pop();
    font_path_buf.push("TEMPSITC.TTF");
    let font_path = font_path_buf.to_str().unwrap().to_owned();

    initial_buttons.push(Box::new(ExampleButton::new("go_left", font_path.clone())));
    initial_buttons.push(Box::new(ExampleButton::new("go_right", font_path)));
    let mut ui = UI::new(&state.canvas, &ttf_context, &texture_creator)?;
    ui.add(initial_buttons);

    let ui_cell = Cell::new(Option::Some(ui));

    state.run(
        |_, event| {
            match event {
                sdl2::event::Event::Quit { .. } => {
                    return {
                        println!("hi");
                        Ok(false)
                    }
                }
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
