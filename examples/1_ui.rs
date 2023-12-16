use std::{
    cell::Cell,
    marker::PhantomData,
    // path::{PathBuf},
    path::PathBuf,
};

use game_engine::{
    core::GameState,
    ui::{
        standard_button_content::{ContentFunctional, TextContent},
        EventHandleResult, UIComponent, UI, standard_button::StandardButton,
    },
};
use sdl2::rect::Rect;

pub fn font() -> String {
    let mut font_path_buf = PathBuf::new();
    font_path_buf.push(file!());
    font_path_buf.pop();
    font_path_buf.push("1_ui_assets");
    font_path_buf.push("TEMPSITC.TTF");
    font_path_buf.to_str().unwrap().to_owned()
}

fn main_menu_button_bound(window_size: (u32, u32), pos: u32) -> Rect {
    let width = 0; // the width of the button is ignored by the text content
    let height = window_size.1 / 8;
    let x = (window_size.0 / 2) as i32;
    let y = (window_size.1 / 2 // bottom half
         + pos * (window_size.1 / 8) // button height
         + (pos + 1) * (window_size.1 / 32)) as i32; // spacing
    Rect::new(x, y, width, height)
}

struct NewGameButtonFunctional<'sdl> {
    _mark: PhantomData<&'sdl ()>,
}

impl<'sdl> ContentFunctional<'sdl> for NewGameButtonFunctional<'sdl> {
    fn released(&mut self) -> EventHandleResult<'sdl> {
        println!("new game button released");
        EventHandleResult::None
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        main_menu_button_bound(window_size, 0)
    }
}

struct LoadGameButtonFunctional<'sdl> {
    _mark: PhantomData<&'sdl ()>,
}

impl<'sdl> ContentFunctional<'sdl> for LoadGameButtonFunctional<'sdl> {
    fn released(&mut self) -> EventHandleResult<'sdl> {
        println!("load game button released");
        EventHandleResult::None
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        main_menu_button_bound(window_size, 1)
    }
}

struct QuitButtonFunctional<'sdl> {
    _mark: PhantomData<&'sdl ()>,
}

impl<'sdl> ContentFunctional<'sdl> for QuitButtonFunctional<'sdl> {
    fn released(&mut self) -> EventHandleResult<'sdl> {
        EventHandleResult::Quit
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        main_menu_button_bound(window_size, 2)
    }
}


fn main() -> Result<(), String> {
    let mut state = GameState::new("ui with layers of buttons", (800u32, 600u32), &[])?;
    let texture_creator = state.canvas.texture_creator();
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut initial_buttons: Vec<Box<dyn UIComponent>> = Vec::new();

    let new_game_functionality = NewGameButtonFunctional { _mark: PhantomData };
    let new_game_content = TextContent::new(
        "New Game".to_string(),
        font(),
        Box::new(new_game_functionality),
    );
    let new_game_button = StandardButton::default_look(Box::new(new_game_content));
    initial_buttons.push(Box::new(new_game_button));

    let load_game_functionality = LoadGameButtonFunctional { _mark: PhantomData };
    let load_game_content = TextContent::new(
        "Load Game".to_string(),
        font(),
        Box::new(load_game_functionality),
    );
    let load_game_button = StandardButton::default_look(Box::new(load_game_content));
    initial_buttons.push(Box::new(load_game_button));

    let quit_functionality = QuitButtonFunctional { _mark: PhantomData };
    let quit_content = TextContent::new(
        "Quit".to_string(),
        font(),
        Box::new(quit_functionality),
    );
    let quit_button = StandardButton::default_look(Box::new(quit_content));
    initial_buttons.push(Box::new(quit_button));

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
