use std::{
    cell::Cell,
    marker::PhantomData,
    // path::{PathBuf},
    path::PathBuf,
};

use game_engine::{
    core::GameState,
    ui::{
        standard_button::StandardButton,
        standard_button_content::{ContentFunctional, TextContent, ImageContent},
        EventHandleResult, UIComponent, UI,
    },
};
use sdl2::{rect::Rect, pixels::Color};

pub fn font() -> String {
    let mut font_path_buf = PathBuf::new();
    font_path_buf.push(file!());
    font_path_buf.pop();
    font_path_buf.push("1_ui_assets");
    font_path_buf.push("TEMPSITC.TTF");
    font_path_buf.to_str().unwrap().to_owned()
}

pub fn test_image() -> String {
    let mut font_path_buf = PathBuf::new();
    font_path_buf.push(file!());
    font_path_buf.pop();
    font_path_buf.push("1_ui_assets");
    font_path_buf.push("test_image.png");
    font_path_buf.to_str().unwrap().to_owned()
}

// place buttons starting at the bottom of the screen, moving upwards with increasing pos
fn bottom_button_bound(window_size: (u32, u32), pos: u32) -> Rect {
    let width = 0; // the width of the button is ignored by the text content
    let height = window_size.1 / 8;
    let x = (window_size.0 / 2) as i32;

    let y = (window_size.1 // start at bottom
        - (pos + 1) * (window_size.1 as f32 / 32f32) as u32 // spacing
        - (pos + 1) * (window_size.1 as f32 / 8f32) as u32 // button height
    ) as i32;
    Rect::new(x, y, width, height)
}

#[derive(Debug)]
enum CharacterSelect {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn character_select_bound(window_size: (u32, u32), character: CharacterSelect) -> Rect {
    let spacing_x = (window_size.0 as f32 / 64f32) as u32;

    // height excluding bottom back button and spacing
    let available_height = window_size.1 - (window_size.1 as f32 / 32f32 + window_size.1 as f32 / 8f32) as u32;

    let spacing_y = (available_height as f32 / 64f32) as u32;
    let width = (window_size.0 - spacing_x * 3) / 2;
    let height = (available_height - spacing_y * 3) / 2;

    let right_side = match character {
        CharacterSelect::TopLeft => false,
        CharacterSelect::TopRight => true,
        CharacterSelect::BottomLeft => false,
        CharacterSelect::BottomRight => true,
    };

    let bottom = match character {
        CharacterSelect::TopLeft => false,
        CharacterSelect::TopRight => false,
        CharacterSelect::BottomLeft => true,
        CharacterSelect::BottomRight => true,
    };

    let x = spacing_x + if right_side { width + spacing_x } else {0};
    let y = spacing_y + if bottom { height + spacing_y } else {0};
    Rect::new(x as i32, y as i32, width, height)
}

fn initial_menu<'sdl>() -> Vec<Box<dyn UIComponent<'sdl> + 'sdl>> {
    struct NewGameButtonFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }

    impl<'sdl> ContentFunctional<'sdl> for NewGameButtonFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::ReplaceLayer(new_game_menu())
        }

        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            bottom_button_bound(window_size, 2)
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
            bottom_button_bound(window_size, 1)
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
            bottom_button_bound(window_size, 0)
        }
    }

    let mut ret: Vec<Box<dyn UIComponent>> = Vec::new();
    let new_game_functionality = NewGameButtonFunctional { _mark: PhantomData };
    let new_game_content = TextContent::new(
        "New Game".to_string(),
        font(),
        Box::new(new_game_functionality),
    );
    let new_game_button = StandardButton::default_look(Box::new(new_game_content));
    ret.push(Box::new(new_game_button));

    let load_game_functionality = LoadGameButtonFunctional { _mark: PhantomData };
    let load_game_content = TextContent::new(
        "Load Game".to_string(),
        font(),
        Box::new(load_game_functionality),
    );
    let load_game_button = StandardButton::default_look(Box::new(load_game_content));
    ret.push(Box::new(load_game_button));

    let quit_functionality = QuitButtonFunctional { _mark: PhantomData };
    let quit_content = TextContent::new("Quit".to_string(), font(), Box::new(quit_functionality));
    let quit_button = StandardButton::default_look(Box::new(quit_content));
    ret.push(Box::new(quit_button));
    ret
}

fn new_game_menu<'sdl>() -> Vec<Box<dyn UIComponent<'sdl> + 'sdl>> {
    let mut ret: Vec<Box<dyn UIComponent>> = Vec::new();

    struct BackButtonFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }

    impl<'sdl> ContentFunctional<'sdl> for BackButtonFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::ReplaceLayer(initial_menu())
        }

        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            bottom_button_bound(window_size, 0)
        }
    }

    let back_functionality = BackButtonFunctional { _mark: PhantomData };
    let back_content = TextContent::new("Back".to_string(), font(), Box::new(back_functionality));
    let back_button = StandardButton::default_look(Box::new(back_content));
    ret.push(Box::new(back_button));
    
    struct TopLeftCharacterFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }
    
    impl<'sdl> ContentFunctional<'sdl> for TopLeftCharacterFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::AddLayer(character_selected_menu(CharacterSelect::TopLeft))
        }
        
        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            character_select_bound(window_size, CharacterSelect::TopLeft)
        }
    }
    let top_left_functionality = TopLeftCharacterFunctional { _mark: PhantomData };
    let top_left_content = ImageContent::new(test_image(), Box::new(top_left_functionality));
    let top_left_button = StandardButton::default_look(Box::new(top_left_content));
    ret.push(Box::new(top_left_button));

    struct TopRightCharacterFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }
    
    impl<'sdl> ContentFunctional<'sdl> for TopRightCharacterFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::AddLayer(character_selected_menu(CharacterSelect::TopRight))
        }
        
        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            character_select_bound(window_size, CharacterSelect::TopRight)
        }
    }
    let top_right_functionality = TopRightCharacterFunctional { _mark: PhantomData };
    let top_right_content = ImageContent::new(test_image(), Box::new(top_right_functionality));
    let top_right_button = StandardButton::default_look(Box::new(top_right_content));
    ret.push(Box::new(top_right_button));

    struct BottomLeftCharacterFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }
    
    impl<'sdl> ContentFunctional<'sdl> for BottomLeftCharacterFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::AddLayer(character_selected_menu(CharacterSelect::BottomLeft))
        }
        
        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            character_select_bound(window_size, CharacterSelect::BottomLeft)
        }
    }
    let bottom_left_functionality = BottomLeftCharacterFunctional { _mark: PhantomData };
    let bottom_left_content = ImageContent::new(test_image(), Box::new(bottom_left_functionality));
    let bottom_left_button = StandardButton::default_look(Box::new(bottom_left_content));
    ret.push(Box::new(bottom_left_button));

    struct BottomRightCharacterFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }
    
    impl<'sdl> ContentFunctional<'sdl> for BottomRightCharacterFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::AddLayer(character_selected_menu(CharacterSelect::BottomRight))
        }
        
        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            character_select_bound(window_size, CharacterSelect::BottomRight)
        }
    }
    let bottom_right_functionality = BottomRightCharacterFunctional { _mark: PhantomData };
    let bottom_right_content = ImageContent::new(test_image(), Box::new(bottom_right_functionality));
    let bottom_right_button = StandardButton::default_look(Box::new(bottom_right_content));
    ret.push(Box::new(bottom_right_button));

    ret
}

fn character_selected_menu<'sdl>(character: CharacterSelect) -> Vec<Box<dyn UIComponent<'sdl> + 'sdl>> {
    let mut ret: Vec<Box<dyn UIComponent>> = Vec::new();

    ret.push(Box::new(game_engine::ui::tint::Tint { color: Color::RGBA(0, 0, 0, 230) }));

    struct BackButtonFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
    }

    impl<'sdl> ContentFunctional<'sdl> for BackButtonFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            EventHandleResult::RemoveLayer
        }

        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            bottom_button_bound(window_size, 0)
        }
    }

    let back_functionality = BackButtonFunctional { _mark: PhantomData };
    let back_content = TextContent::new("Back".to_string(), font(), Box::new(back_functionality));
    let back_button = StandardButton::default_look(Box::new(back_content));
    ret.push(Box::new(back_button));

    struct GoButtonFunctional<'sdl> {
        _mark: PhantomData<&'sdl ()>,
        character: CharacterSelect
    }

    impl<'sdl> ContentFunctional<'sdl> for GoButtonFunctional<'sdl> {
        fn released(&mut self) -> EventHandleResult<'sdl> {
            println!("lets go {:?}", self.character);
            EventHandleResult::None
        }

        fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
            bottom_button_bound(window_size, 1)
        }
    }

    let go_functionality = GoButtonFunctional { _mark: PhantomData, character };
    let go_content = TextContent::new("Start".to_string(), font(), Box::new(go_functionality));
    let go_button = StandardButton::default_look(Box::new(go_content));
    ret.push(Box::new(go_button));

    ret
}

fn main() -> Result<(), String> {
    let mut state = GameState::new("ui with layers of buttons", (400u32, 600u32), &[])?;
    let texture_creator = state.canvas.texture_creator();
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut ui = UI::new(&state.canvas, &ttf_context, &texture_creator)?;
    ui.add(initial_menu());

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
