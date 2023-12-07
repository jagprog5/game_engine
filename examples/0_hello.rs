extern crate game_engine;
use core::panic;
use std::path::PathBuf;
use rand::prelude::*;


use game_engine::{
    GameState, Persistent, PersistentSpawnChanges, Volatile, VolatileSpawn, VolatileSpawnChanges, PersistentRef,
};

fn central_rand(radius: f32) -> (f32, f32) {
    let mut rng = rand::thread_rng();
    let theta = rng.gen_range(0f32..2f32 * std::f32::consts::PI);
    (theta.cos() * radius, theta.sin() * radius)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PrimarySquare {
    // colour gradient profile. starts as:
    r: bool,
    g: bool,
    b: bool,

    // coords origin is center of screen, throughout this example
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,

    #[serde(skip)]
    x_rate: f32,
    #[serde(skip)]
    y_rate: f32,
    #[serde(skip)]
    dx_rate: f32,
    #[serde(skip)]
    dy_rate: f32,
    #[serde(skip)]
    d_dampener: f32,
}

impl PrimarySquare {
    const SIZE: f32 = 20.;
    fn new() -> Self {
        let (x, y) = central_rand(200f32);
        let dist = (x.powi(2) + y.powi(2)).sqrt();
        let (dx, dy) = (-y / dist, x / dist);
        let mut rng = rand::thread_rng();
        PrimarySquare {
            r: rng.gen_bool(0.5),
            g: rng.gen_bool(0.5),
            b: rng.gen_bool(0.5),
            x,
            y,
            dx,
            dy,
            x_rate: 0f32,
            y_rate: 0f32,
            dx_rate: 0f32,
            dy_rate: 0f32,
            d_dampener: 1f32,
        }
    }
}

#[typetag::serde]
impl Persistent for PrimarySquare {
    fn generate_rate(&mut self, _state: &game_engine::GameState) {
        self.x_rate = self.dx;
        self.y_rate = self.dy;

        let mut r = (self.x.powi(2) + self.y.powi(2)).sqrt();
        r = r.max(5f32);

        // accelerate based on inverse square of distance
        self.dx_rate = -self.x / r.powi(2);
        self.dy_rate = -self.y / r.powi(2);

        // there's some instability which causes it to fling off into infinite
        // over time. to prevent this, a speed dampener is applied based on distance
        let r_div = (2f32 * 700f32.powi(2)).sqrt();
        self.d_dampener = 1f32 - r / (r_div) * 0.01;

        // the dampener reduces all velocity (inclusing rotational component).
        // adding a tiny amount to keep things spinning spin in opposite
        // direction over time
        let (x_h, y_h) = (self.x / r, self.y / r);
        self.dx_rate += -y_h * 0.0002;
        self.dy_rate += x_h * 0.0002;
    }

    fn apply_rate(&mut self) {
        self.x += self.x_rate;
        self.dx += self.dx_rate;
        self.y += self.y_rate;
        self.dy += self.dy_rate;

        self.dx *= self.d_dampener;
        self.dy *= self.d_dampener;
    }

    fn apply_spawns(&self) -> PersistentSpawnChanges {
        let mut volatile_spawns: Vec<(&'static str, Vec<VolatileSpawn>)> = Vec::new();
        volatile_spawns.push((OBJECTS, vec![Box::new(PrimarySquareTail::new(&self))]));
        PersistentSpawnChanges {
            alive: true,
            volatile_spawns,
            persistent_spawns: Vec::new(),
        }
    }

    /// draw to the screen
    fn render(&self, _canvas: &mut sdl2::render::WindowCanvas, _window_size: (u32, u32)) {
        // this entity drawn entirely from particle effects it emitts
    }
}

// =================================================================================================

struct PrimarySquareTail {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    x_rate: f32,
    y_rate: f32,
    dx_rate: f32,
    dy_rate: f32,
    r: bool,
    g: bool,
    b: bool,
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
            x_rate: 0f32,
            y_rate: 0f32,
            dx_rate: 0f32,
            dy_rate: 0f32,
            r: from.r,
            g: from.g,
            b: from.b,
            alpha: 255,
        }
    }
}

impl Volatile for PrimarySquareTail {
    fn generate_rate(&mut self, _state: &game_engine::GameState) {
        self.x_rate = self.dx;
        self.y_rate = self.dy;
        // deviate more and more as the particles expire
        let progress = (self.alpha) as f32 / 255f32;
        let (drift_x, drift_y) = central_rand(0.1f32 * (1f32 - progress));
        self.dx_rate = drift_x;
        self.dy_rate = drift_y;
    }

    fn apply_rate(&mut self) {
        self.x += self.x_rate;
        self.dx += self.dx_rate;
        self.y += self.y_rate;
        self.dy += self.dy_rate;
        self.alpha -= 1;
    }

    fn apply_spawns(&self) -> VolatileSpawnChanges {
        VolatileSpawnChanges {
            alive: self.alpha != 0,
            volatile_spawns: Vec::new(),
        }
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32)) {
        let progress_on = (self.alpha) as f32 / 255f32; // from 1 (inclusive) to 0 (exclusive)
        let progress_off = 1f32 - progress_on;

        let size = PrimarySquare::SIZE * progress_on;
        let red = (255f32 * if self.r { progress_on } else { progress_off }) as u8;
        let green = (255f32 * if self.g { progress_on } else { progress_off }) as u8;
        let blue = (255f32 * if self.b { progress_on } else { progress_off }) as u8;
        let alpha = (100f32 * progress_on) as u8;
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

// =================================================================================================

// this point randomly selects and follows any object (including itself! important to test self refs)
#[derive(serde::Serialize, serde::Deserialize)]
struct Follower {
    #[serde(skip)]
    followee: Option<PersistentRef>,
    x: f32,
    y: f32,
    countdown: usize,
}

impl Follower {
    const SIZE: f32 = 10.;
    const MOVE_SPEED: f32 = 2.;
    fn new() -> Self {
        Self {
            followee: None,
            x: 0f32,
            y: 0f32,
            countdown: 0
        }
    }

    fn get_follow_pos(f: &Option<PersistentRef>) -> (f32, f32) {
        if let None = f {
            // could be that OBJECTS layer is empty. can't obtain a new
            // followee. this won't happen since in this case self is always in
            // that layer, making it never empty.
            return (0f32, 0f32);
        }

        let rc = match f.clone().unwrap().upgrade() {
            Some(rc) => rc, // PersistentEntity successfully gained
            None => {
                // followee has despawned. should never happen. the ref was set
                // at generate_rate, and this function is called in apply_rate.
                // despawn doesn't happen between those two phases. it happens
                // after apply_rate, in apply_spawns
                return (0f32, 0f32)
            },
        };

        let e = rc.take();
        
        if let None = e {
            // this happens if e is self! already taken by game state before
            // calling update functions
            return (0f32, 0f32)
        }
        let e = e.unwrap();
        let e = match e.downcast::<Follower>() {
            Ok(e) => {
                let r = (e.x, e.y);
                rc.set(Some(e));
                return r;
            },
            Err(e) => e,
        };
        let _e = match e.downcast::<PrimarySquare>() {
            Ok(e) => {
                let r = (e.x, e.y);
                rc.set(Some(e));
                return r;
            },
            Err(e) => e,
        };
        panic!("get_follow_pos not implemented for followee type");
    }
}

#[typetag::serde]
impl Persistent for Follower {
    fn save_entity_references(&self) -> Vec<Option<PersistentRef>> {
        vec![self.followee.clone()]
    }

    fn load_entity_references(&mut self, v: Vec<Option<PersistentRef>>) {
        if v.len() != 1 {
            panic!("follower requires one persistent ref to be loaded from the save file")
        }
        let mut i = v.into_iter();
        self.followee = i.next().unwrap();
    }

    fn generate_rate(&mut self, state: &GameState) {
        let mut needs_new_followee = false;
        if self.countdown == 0 {
            // periodically get new follower
            needs_new_followee = true;
        } else if self.followee.is_none() {
            // this will be none on one of two conditions.
            // 1. this is the first time generate_rate is called
            // 2. the save file was tampered with
            needs_new_followee = true;
        } else if let None = self.followee.clone().unwrap().upgrade() {
            // followee has been freed (it is no longer alive)
            needs_new_followee = true;
        }        

        if needs_new_followee {
            let entities = state.get_persistents(OBJECTS);
            if !entities.is_empty() {
                let random_index = rand::thread_rng().gen_range(0..entities.len());
                let random_element = &entities[random_index];
                self.followee = Some(std::rc::Rc::downgrade(&random_element.rc));
            }
        }
    }

    fn apply_rate(&mut self) {
        if self.countdown == 0 {
            self.countdown = 5 * 60; // 60 downto 1
        } else {
            self.countdown -= 1;
        }

        let (goal_x, goal_y) = Follower::get_follow_pos(&self.followee);
        let mag = ((goal_x - self.x).powi(2) + (goal_y - self.y).powi(2)).sqrt();
        if mag < Follower::MOVE_SPEED {
            // prevent jitter
            self.x = goal_x;
            self.y = goal_y;
        } else {
            let dx = (goal_x - self.x) / mag * Follower::MOVE_SPEED;
            let dy = (goal_y - self.y) / mag * Follower::MOVE_SPEED;
            self.x += dx;
            self.y += dy;
        }
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32)) {
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 255, 100));
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                (self.x - Follower::SIZE / 2.) as i32 + window_size.0 as i32 / 2,
                (self.y - Follower::SIZE / 2.) as i32 + window_size.1 as i32 / 2,
                Follower::SIZE as u32,
                Follower::SIZE as u32,
            ))
            .unwrap();
    }
}

// =================================================================================================

fn get_save_path() -> String {
    let mut save_path: PathBuf = file!().into();
    save_path.pop();
    save_path.push("0_hello_save_file.save");
    save_path.to_str().unwrap().to_owned()
}

const OBJECTS: &'static str = "objects";
const RENDER_ORDER: &'static [&'static str] = &[OBJECTS];

fn main() -> Result<(), String> {
    let save_file_path: String = get_save_path();

    fn populate_initial_entities(state: &mut GameState) {
        for _ in 0..5 {
            state.spawn_persistent(Box::new(PrimarySquare::new()), OBJECTS);
        }
        state.spawn_persistent(Box::new(Follower::new()), OBJECTS);
    }

    let mut state = GameState::new("controls: s, l, r, esc", (800u32, 600u32), RENDER_ORDER)?;
    // check if save file already exists
    if std::fs::metadata(save_file_path.clone()).is_ok() {
        println!("loading save");
        state.load(save_file_path.clone())?;
    } else {
        populate_initial_entities(&mut state);
    }
    state.run(|state, event| {
        match event {
            sdl2::event::Event::Quit { .. }
            | sdl2::event::Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Escape),
                ..
            } => return Ok(false),
            sdl2::event::Event::KeyUp {
                keycode: Some(sdl2::keyboard::Keycode::S),
                ..
            } => {
                state.save(save_file_path.clone())?;
                println!("manual save");
            }
            sdl2::event::Event::KeyUp {
                keycode: Some(sdl2::keyboard::Keycode::L),
                ..
            } => {
                if std::fs::metadata(save_file_path.clone()).is_ok() {
                    state.load(save_file_path.clone())?;
                    println!("manual load");
                }
            }
            sdl2::event::Event::KeyUp {
                keycode: Some(sdl2::keyboard::Keycode::R),
                ..
            } => {
                state.clear_persistent();
                populate_initial_entities(state);
                println!("reset");
            }
            _ => {}
        }
        Ok(true)
    })?;
    println!("save on exit");
    state.save(save_file_path)?;
    Ok(())
}
