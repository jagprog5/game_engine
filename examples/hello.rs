extern crate game_engine;
use std::cell::Cell;
use rand::prelude::*;

use game_engine::Persistent;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[derive(serde::Serialize, serde::Deserialize)]
struct CentralSquare {
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

impl CentralSquare {
    fn new() -> Self {
        let mut rng = rand::thread_rng();

        let x = rng.gen_range(-100.0..=100.0);
        let y = rng.gen_range(-100.0..=100.0);
        let dx = rng.gen_range(-1.0..=1.0);
        let dy = rng.gen_range(-1.0..=1.0);

        let x_rate = Cell::new(0.0);
        let y_rate = Cell::new(0.0);
        let dx_rate = Cell::new(0.0);
        let dy_rate = Cell::new(0.0);

        CentralSquare {
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

impl CentralSquare {
    const SIZE: f32 = 20.;
}

#[typetag::serde]
impl Persistent for CentralSquare {
    fn generate_rate(&self, _persistent: &Vec<Box<dyn Persistent>>) {
        self.x_rate.set(self.dx);
        self.y_rate.set(self.dy);

        let r = (self.x.powi(2) + self.y.powi(2)).sqrt();

        self.dx_rate.set(-self.x / r.powi(2));
        self.dy_rate.set(-self.y / r.powi(2));
    }

    fn apply_rate(&mut self) {
        self.x += self.x_rate.get();
        self.dx += self.dx_rate.get();
        self.y += self.y_rate.get();
        self.dy += self.dy_rate.get();

        // decay:
        self.dx *= 0.9995;
        self.dy *= 0.9995;
    }

    /// draw to the screen
    fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        canvas.set_draw_color(sdl2::pixels::Color::GREEN);
        // let size = canvas.output_size();
        canvas
            .draw_rect(sdl2::rect::Rect::new(
                (self.x - CentralSquare::SIZE / 2.) as i32 + WIDTH as i32 / 2,
                (self.y - CentralSquare::SIZE / 2.) as i32 + HEIGHT as i32 / 2,
                CentralSquare::SIZE as u32,
                CentralSquare::SIZE as u32,
            ))
            .unwrap();
    }

    fn alive(&self) -> bool {
        return true;
    }
}

fn main() -> Result<(), String> {
    let mut state = game_engine::GameState::new("first example", (WIDTH, HEIGHT))?;
    state.persistent.push(Box::new(CentralSquare::new()));
    state.run();
    Ok(())
}
