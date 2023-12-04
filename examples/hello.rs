extern crate game_engine;
use rand::prelude::*;
use std::{cell::Cell, path::PathBuf};

use game_engine::{Entity, GameState, Persistent, Volatile};

fn central_rand(radius: f32) -> (f32, f32) {
    let mut rng = rand::thread_rng();
    let theta = rng.gen_range(0f32..2f32 * std::f32::consts::PI);
    (theta.cos() * radius, theta.sin() * radius)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PrimarySquare {
    // coords origin is center of screen, throughout this example
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,

    #[serde(skip)]
    x_rate: Cell<f32>,
    #[serde(skip)]
    y_rate: Cell<f32>,
    #[serde(skip)]
    dx_rate: Cell<f32>,
    #[serde(skip)]
    dy_rate: Cell<f32>,
}

impl PrimarySquare {
    fn new() -> Self {
        let (x, y) = central_rand(200f32);
        let dist = (x.powi(2) + y.powi(2)).sqrt();
        let (dx, dy) = (-y / dist, x / dist);
        // let (dx, dy) = central_rand(1f32);
        // let 
        let x_rate = Cell::new(0.0);
        let y_rate = Cell::new(0.0);
        let dx_rate = Cell::new(0.0);
        let dy_rate = Cell::new(0.0);

        PrimarySquare {
            x,
            y,
            dx,
            dy,
            x_rate,
            y_rate,
            dx_rate,
            dy_rate,
        }
    }
}

impl PrimarySquare {
    const SIZE: f32 = 20.;
}

impl Volatile for PrimarySquare {
    fn generate_rate(&self, _state: &game_engine::GameState) {
        self.x_rate.set(self.dx);
        self.y_rate.set(self.dy);

        let mut r = (self.x.powi(2) + self.y.powi(2)).sqrt();
        r = r.max(0.00001);
        self.dx_rate.set(-self.x / r.powi(2));
        self.dy_rate.set(-self.y / r.powi(2));
    }

    fn apply_rate(&mut self) -> (bool, Vec<(String, Vec<Entity>)>) {
        self.x += self.x_rate.get();
        self.dx += self.dx_rate.get();
        self.y += self.y_rate.get();
        self.dy += self.dy_rate.get();

        // decay. there's some instability so otherwise it
        // would fling off into infinite over time
        self.dx *= 0.9995;
        self.dy *= 0.9995;

        let mut to_spawn: Vec<(String, Vec<Entity>)> = Vec::new();
        to_spawn.push((
            "objects".to_owned(),
            vec![Entity::Volatile(Box::new(PrimarySquareTail::new(&self)))],
        ));
        (true, to_spawn)
    }

    /// draw to the screen
    fn render(&self, _canvas: &mut sdl2::render::WindowCanvas, _window_size: (u32, u32)) {
        // this entity drawn entirely from particle effects it emitts
    }
}

#[typetag::serde]
impl Persistent for PrimarySquare {}

struct PrimarySquareTail {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    x_rate: Cell<f32>,
    y_rate: Cell<f32>,
    dx_rate: Cell<f32>,
    dy_rate: Cell<f32>,
    alpha: u8,
}

impl PrimarySquareTail {
    fn new(from: &PrimarySquare) -> Self {
        let (drift_x, drift_y) = central_rand(0.2f32);
        Self {
            x: from.x,
            y: from.y,
            dx: from.dx + drift_x,
            dy: from.dy + drift_y,
            x_rate: Cell::new(0f32),
            y_rate: Cell::new(0f32),
            dx_rate: Cell::new(0f32),
            dy_rate: Cell::new(0f32),
            alpha: 255,
        }
    }
}

impl Volatile for PrimarySquareTail {
    fn generate_rate(&self, _state: &game_engine::GameState) {
        self.x_rate.set(self.dx);
        self.y_rate.set(self.dy);
        // deviate more and more as the particles expire
        let progress = (self.alpha) as f32 / 255f32;
        let (drift_x, drift_y) = central_rand(0.1f32 * (1f32 - progress));
        self.dx_rate.set(drift_x);
        self.dy_rate.set(drift_y);
    }

    fn apply_rate(&mut self) -> (bool, Vec<(String, Vec<Entity>)>) {
        self.x += self.x_rate.get();
        self.dx += self.dx_rate.get();
        self.y += self.y_rate.get();
        self.dy += self.dy_rate.get();
        self.alpha -= 1;
        (self.alpha != 0, Vec::new())
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32)) {
        let progress = (self.alpha) as f32 / 255f32; // from 1 (inclusive) to 0 (exclusive)
        let size = PrimarySquare::SIZE * progress;
        let red = (255f32 * (1f32 - progress)) as u8;
        let green = (255f32 * progress) as u8;
        let blue = (255f32 * (1f32 - progress)) as u8;
        let alpha = (255f32 * progress) as u8;
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(red, green, blue, alpha));
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                (self.x - size / 2.) as i32 + window_size.0 as i32 / 2,
                (self.y - size / 2.) as i32 + window_size.1 as i32 / 2,
                size as u32,
                size as u32,
            ))
            .unwrap();
    }
}

fn get_save_path() -> String {
    let mut save_path: PathBuf = file!().into();
    save_path.pop();
    save_path.push("hello_save_file.bin");
    save_path.to_str().unwrap().to_owned()
}

fn main() -> Result<(), String> {
    let mut state = GameState::new(
        "HELLO WORLD: graphics, persistent / volatile",
        (800u32, 600u32),
        vec!["objects".to_owned()],
    )?;
    // check if save file already exists
    if std::fs::metadata(get_save_path()).is_ok() {
        println!("recovering last save");
        state.load(get_save_path())?;
    } else {
        state
            .save_state
            .layers
            .get_mut("objects")
            .unwrap()
            .push(Entity::Persistent(Box::new(PrimarySquare::new())));
    }
    state.run(|event| {
        match event {
            sdl2::event::Event::Quit { .. }
            | sdl2::event::Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Escape),
                ..
            } => return false,
            _ => {}
        }
        true
    });
    state.save(get_save_path())?;
    Ok(())
}
