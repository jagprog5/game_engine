use std::{
    cell::Cell,
    marker::PhantomData,
    // path::{PathBuf},
    path::PathBuf,
};

use game_engine::{
    core::GameState,
    ui::{Button, EventHandleResult, FontCache, UIComponent, UIState, UI},
};
use sdl2::{
    pixels::Color,
    rect::Rect,
    render::{Texture, TextureCreator, WindowCanvas},
    video::WindowContext,
};

#[derive(PartialEq, Eq)]
// button state for rendering, changed by sdl events
enum FocusState {
    Idle,
    Hovered,
    Pressed,
}

#[derive(PartialEq, Eq)]
// indicates properties for MainMenuButton
enum MainMenuButtonProps<'sdl> {
    NewGame(PhantomData<&'sdl ()>),
    LoadGame,
    Quit,
}

impl<'sdl> MainMenuButtonProps<'sdl> {
    pub fn text(&self) -> &'static str {
        match self {
            MainMenuButtonProps::NewGame(_) => "New Game",
            MainMenuButtonProps::LoadGame => "Load Game",
            MainMenuButtonProps::Quit => "Quit",
        }
    }

    pub fn font() -> String {
        let mut font_path_buf = PathBuf::new();
        font_path_buf.push(file!());
        font_path_buf.pop();
        font_path_buf.push("TEMPSITC.TTF");
        font_path_buf.to_str().unwrap().to_owned()
    }

    // u16 since this influences the font size
    pub fn height(window_height: u32) -> u16 {
        // bottom half of the screen is buttons
        // 3/4 of that is filled by buttons, the final 1/4 is for spacing
        (window_height / 8).try_into().unwrap()
    }

    // u16 since this influences the font size
    pub fn border_width() -> u16 {
        10u16
    }

    pub fn y(&self, window_height: u32) -> u32 {
        let pos: u32 = match self {
            MainMenuButtonProps::NewGame(_) => 0,
            MainMenuButtonProps::LoadGame => 1,
            MainMenuButtonProps::Quit => 2,
        };
        (window_height / 2) // bottom half
         + pos * u32::from(Self::height(window_height)) // button height
         + (pos + 1) * (window_height / 32) // spacing
    }

    fn released(&mut self) -> EventHandleResult<'sdl> {
        match self {
            MainMenuButtonProps::NewGame(_) => todo!(),
            MainMenuButtonProps::LoadGame => todo!(),
            MainMenuButtonProps::Quit => EventHandleResult::Quit,
        }
    }
}

struct MainMenuButton<'sdl> {
    // rendered text. swapped out per call to resize
    rendered_text: Option<Texture<'sdl>>,

    bound: Rect,
    focus_state: FocusState,

    which: MainMenuButtonProps<'sdl>,
}

impl<'sdl> MainMenuButton<'sdl> {
    fn new(which: MainMenuButtonProps<'sdl>) -> Self {
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

impl<'sdl> UIComponent<'sdl> for MainMenuButton<'sdl> {
    fn process(&mut self, ui_state: &UIState, e: &sdl2::event::Event) -> EventHandleResult<'sdl> {
        Button::<'sdl>::process(self, ui_state, e)
    }

    fn render(&self, canvas: &mut WindowCanvas) {
        // background
        canvas.set_draw_color(match self.focus_state {
            FocusState::Idle => Color::RGBA(100, 100, 100, 30),
            FocusState::Hovered => Color::RGBA(100, 100, 100, 50),
            FocusState::Pressed => Color::RGBA(100, 100, 100, 80),
        });
        canvas.fill_rect(self.bound).unwrap();

        let border_width_divisions: u8 = 3;
        let division_width_u32 = u32::from(MainMenuButtonProps::border_width())
            / (u32::from(border_width_divisions) + 1);
        let division_width_i32 = i32::from(MainMenuButtonProps::border_width())
            / (i32::from(border_width_divisions) + 1);

        for i in 0..border_width_divisions {
            // outer border
            let color = 200 - i * 50;
            canvas.set_draw_color(Color::RGB(color, color, color));
            canvas // top
                .fill_rect(Rect::new(
                    self.bound.x + i32::from(i) * division_width_i32,
                    self.bound.y + i32::from(i) * division_width_i32,
                    self.bound.w as u32 - u32::from(i) * division_width_u32 * 2,
                    division_width_u32,
                ))
                .unwrap();
            canvas // right
                .fill_rect(Rect::new(
                    self.bound.x + self.bound.w - (1 + i32::from(i)) * division_width_i32,
                    self.bound.y + i32::from(i) * division_width_i32,
                    division_width_u32,
                    (self.bound.h as u32)
                        .checked_sub(u32::from(i) * division_width_u32 * 2)
                        .unwrap_or(self.bound.h as u32),
                ))
                .unwrap();
            canvas // bottom
                .fill_rect(Rect::new(
                    self.bound.x + i32::from(i) * division_width_i32,
                    self.bound.y + self.bound.h - (1 + i32::from(i)) * division_width_i32,
                    self.bound.w as u32 - u32::from(i) * division_width_u32 * 2,
                    division_width_u32,
                ))
                .unwrap();
            canvas // left
                .fill_rect(Rect::new(
                    self.bound.x + i32::from(i) * division_width_i32,
                    self.bound.y + i32::from(i) * division_width_i32,
                    division_width_u32,
                    (self.bound.h as u32)
                        .checked_sub(u32::from(i) * division_width_u32 * 2)
                        .unwrap_or(self.bound.h as u32),
                ))
                .unwrap();
        }

        let text_texture = self.rendered_text.as_ref().unwrap();
        let border_width = MainMenuButtonProps::border_width();
        canvas
            .copy(
                text_texture,
                None,
                Rect::new(
                    self.bound.x + i32::from(border_width),
                    self.bound.y + i32::from(border_width),
                    self.bound.w as u32 // safe since resize never sets negative
                     - u32::from(border_width) * 2, // safe sub since resize added same amount
                    (self.bound.h as u32)
                        .checked_sub(u32::from(border_width) * 2)
                        .unwrap_or(self.bound.h as u32),
                ),
            )
            .unwrap();
    }

    fn resize(
        &mut self,
        window_size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_cache: &mut FontCache,
    ) {
        let font_path = MainMenuButtonProps::font();
        let height = MainMenuButtonProps::height(window_size.1);
        let border_width = MainMenuButtonProps::border_width();
        let font_size = height.checked_sub(border_width * 2).unwrap_or(height);
        let text = self.which.text();

        let font_rc = font_cache.get(font_path, font_size);

        let surface = font_rc
            .render(text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();

        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .unwrap();

        let text_width = texture.query().width;
        let width = text_width + u32::from(border_width) * 2;
        self.rendered_text = Some(texture);

        let x = (window_size.0 / 2).checked_sub(width / 2).unwrap_or(0);
        let x = x as i32; // safe since MSB can't be 1
        let y: i32 = self.which.y(window_size.1).try_into().unwrap(); // will panic on very large window
        self.bound = Rect::new(x, y, width, u32::from(height));
    }
}

impl<'sdl> Button<'sdl> for MainMenuButton<'sdl> {
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
        self.which.released()
    }
}

fn main() -> Result<(), String> {
    let mut state = GameState::new("ui with layers of buttons", (800u32, 600u32), &[])?;
    let texture_creator = state.canvas.texture_creator();
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut initial_buttons: Vec<Box<dyn UIComponent>> = Vec::new();

    initial_buttons.push(Box::new(MainMenuButton::new(MainMenuButtonProps::NewGame(
        PhantomData,
    ))));
    initial_buttons.push(Box::new(MainMenuButton::new(MainMenuButtonProps::LoadGame)));
    initial_buttons.push(Box::new(MainMenuButton::new(MainMenuButtonProps::Quit)));
    let mut ui = UI::new(&state.canvas, &ttf_context, &texture_creator)?;
    ui.add(initial_buttons);

    let ui_cell = Cell::new(Option::Some(ui));

    state.run(
        |_, event| {
            match event {
                sdl2::event::Event::Quit { .. } => return Ok(false),
                _ => {}
            }
            let mut ui = ui_cell.take().unwrap();
            let ret = ui.process(event);
            ui_cell.set(Some(ui));
            Ok(ret)
        },
        |canvas| {
            let ui = ui_cell.take().unwrap();
            ui.render(canvas);
            ui_cell.set(Some(ui));
        },
    )?;
    Ok(())
}
