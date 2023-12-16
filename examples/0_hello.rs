/*
This demo ensure that the core is working.

It features:
    - animation, graphics, basic keyboard input
    - persistent and volatile entities
    - persistence (save file) in general
    - persistent references which can be:
        - circular (points to self)
        - pointing to elements which have despawned
*/

extern crate game_engine;
use core::panic;
use rand::prelude::*;
use std::path::PathBuf;

use game_engine::core::{
    GameState, MaybePersistentRef, Persistent, PersistentRef, PersistentRefPromotionResult,
    PersistentSpawn, PersistentSpawnChanges, Volatile, VolatileSpawn, VolatileSpawnChanges, LivelinessStatus,
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

    // starts at 1 on spawn and fades up to max.
    // used for gradual increase on spawn
    fade_in_alpha: u8,

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
    const REPLACE_CHANCE: f64 = 0.0005;
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
            fade_in_alpha: 1,
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
    fn generate_rate(&mut self, _state: &GameState) {
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
        if self.fade_in_alpha != u8::MAX {
            self.fade_in_alpha += 1;
        }

        self.x += self.x_rate;
        self.dx += self.dx_rate;
        self.y += self.y_rate;
        self.dy += self.dy_rate;

        self.dx *= self.d_dampener;
        self.dy *= self.d_dampener;
    }

    fn apply_spawns(&self) -> PersistentSpawnChanges {
        let replace_self: bool = rand::thread_rng().gen_bool(Self::REPLACE_CHANCE);

        let mut persistent_spawns: Vec<(&'static str, Vec<PersistentSpawn>)> = Vec::new();
        if replace_self {
            persistent_spawns.push((OBJECTS, vec![Box::new(PrimarySquare::new())]))
        }

        let mut volatile_spawns: Vec<(&'static str, Vec<VolatileSpawn>)> = Vec::new();
        volatile_spawns.push((OBJECTS, vec![Box::new(PrimarySquareTail::new(&self))]));
        PersistentSpawnChanges {
            alive: LivelinessStatus::new(!replace_self),
            volatile_spawns,
            persistent_spawns,
        }
    }

    /// draw to the screen
    fn render(&self, _canvas: &mut sdl2::render::WindowCanvas) {
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
            alpha: from.fade_in_alpha,
        }
    }
}

impl Volatile for PrimarySquareTail {
    fn generate_rate(&mut self, _state: &GameState) {
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
            alive: LivelinessStatus::new(self.alpha != 0),
            volatile_spawns: Vec::new(),
        }
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        let window_size = canvas.output_size().unwrap();
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
    followee: MaybePersistentRef,
    x: f32,
    y: f32,
    speed: f32,
    countdown: usize,
}

impl Follower {
    const SIZE: f32 = 2.;
    const MIN_SPEED: f32 = 2.;
    const MAX_SPEED_EXCLUSIVE: f32 = 10.;
    const COUNTDOWN_RESET: usize = 5 * GameState::GOAL_FPS as usize;

    fn new() -> Self {
        Self {
            followee: MaybePersistentRef::None,
            x: 0f32,
            y: 0f32,
            speed: rand::thread_rng().gen_range(Self::MIN_SPEED..Self::MAX_SPEED_EXCLUSIVE),
            countdown: rand::thread_rng().gen_range(0..Self::COUNTDOWN_RESET),
        }
    }

    /// holds in place with self reference or if followee has despawned or not been set.
    /// otherwise yields the entities position
    fn get_follow_pos(&self) -> (f32, f32) {
        let f = &self.followee;
        let weak_ref = match f {
            // could be that OBJECTS layer is empty. can't obtain a new
            // followee. this won't happen since in this case self is always in
            // that layer, making it never empty.
            MaybePersistentRef::None => return (self.x, self.y),
            // followee despawned. hold in place until countdown runs out
            MaybePersistentRef::Despawned => return (self.x, self.y),
            MaybePersistentRef::Some(e) => e,
        };

        let (e_position, e) = match weak_ref.get() {
            PersistentRefPromotionResult::Despawned => return (self.x, self.y),
            // self reference. go to origin
            PersistentRefPromotionResult::Taken => return (self.x, self.y),
            PersistentRefPromotionResult::Some(e_position, e) => (e_position, e),
        };

        // downcast to concrete type and get position
        let e = match e.downcast::<Follower>() {
            Ok(e) => {
                let ret = (e.x, e.y);
                // return the entitiy back into its position
                PersistentRef::set((e_position, e));
                return ret;
            }
            Err(e) => e,
        };
        let _e = match e.downcast::<PrimarySquare>() {
            Ok(e) => {
                let ret = (e.x, e.y);
                PersistentRef::set((e_position, e));
                return ret;
            }
            Err(e) => e,
        };
        panic!("get_follow_pos not implemented for followee type");
    }
}

#[typetag::serde]
impl Persistent for Follower {
    fn save_entity_references(&self) -> Vec<MaybePersistentRef> {
        vec![self.followee.clone()]
    }

    fn load_entity_references(&mut self, v: Vec<MaybePersistentRef>) {
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
        } else if let MaybePersistentRef::None = self.followee {
            needs_new_followee = true;
        } else if let MaybePersistentRef::Despawned = self.followee {
            needs_new_followee = true;
        }

        if !needs_new_followee {
            // nothing else is done in generate rate except setting the followee if needed
            return;
        }

        let entities = state.get_persistents(OBJECTS);

        // do not get a new followee if it's impossible to do so
        if entities.is_empty() {
            return;
        }

        // don't stay on the same entity more than once. loop until new one found
        loop {
            let random_index = rand::thread_rng().gen_range(0..entities.len());
            let random_entity = &entities[random_index];

            // should the new one be used? or should it do another
            // iteration and find another random one
            let use_random_entity: bool = match &self.followee {
                // if the current one is
                MaybePersistentRef::None => true,
                MaybePersistentRef::Despawned => true,
                MaybePersistentRef::Some(persistent_ref) => {
                    if entities.len() <= 1 {
                        // impossible to find different followee
                        true
                    } else {
                        // check if this followee is different that the current one
                        match persistent_ref.0.upgrade() {
                            Some(e) => {
                                // only condition in which another iteration
                                // happens is when entities.len() > 1 and the
                                // randomly selected entitiy is the same as the
                                // followee
                                !std::rc::Rc::ptr_eq(&e, &random_entity.0)
                            }
                            None => true,
                        }
                    }
                }
            };

            if use_random_entity {
                let weak = std::rc::Rc::downgrade(&random_entity.0);
                self.followee = MaybePersistentRef::Some(PersistentRef(weak));
                break;
            }
        }
    }

    fn apply_rate(&mut self) {
        if self.countdown == 0 {
            self.countdown = Self::COUNTDOWN_RESET;
        } else {
            // each follow occurs for Self::COUNTDOWN_RESET downto 1 frames (inclusive)
            self.countdown -= 1;
        }

        let (goal_x, goal_y) = Follower::get_follow_pos(&self);
        let mag = ((goal_x - self.x).powi(2) + (goal_y - self.y).powi(2)).sqrt();
        if mag < self.speed {
            // prevent jitter
            self.x = goal_x;
            self.y = goal_y;
        } else {
            let dx = (goal_x - self.x) / mag * self.speed;
            let dy = (goal_y - self.y) / mag * self.speed;
            self.x += dx;
            self.y += dy;
        }
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        let window_size = canvas.output_size().unwrap();
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 255, 50));
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
        for _ in 0..700 {
            state.spawn_persistent(Box::new(Follower::new()), OBJECTS);
        }
        for _ in 0..5 {
            state.spawn_persistent(Box::new(PrimarySquare::new()), OBJECTS);
        }
    }

    let mut state = GameState::new("controls: s, l, r, esc", (800u32, 600u32), RENDER_ORDER)?;
    // check if save file already exists
    if std::fs::metadata(save_file_path.clone()).is_ok() {
        println!("loading save");
        state.load(save_file_path.clone())?;
    } else {
        populate_initial_entities(&mut state);
    }
    state.run(
        |state, event| {
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
        },
        |_| {},
    )?;
    println!("save on exit");
    state.save(save_file_path)?;
    Ok(())
}
