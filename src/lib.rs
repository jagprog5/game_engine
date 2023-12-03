use std::default;

extern crate sdl2;

pub struct GameState {
    /// things which are part of the game loop and saved. items, enemies, etc.
    pub persistent: Vec<Box<dyn Persistent>>,

    /// things which are part of the game loop and NOT saved. particle effects
    /// drawn UNDERNEATH persistent object
    pub under_volatile: Vec<Box<dyn Volatile>>,

    /// things which are part of the game loop and NOT saved. particle effects
    /// drawn OVER persistent object
    pub over_volatile: Vec<Box<dyn Volatile>>,

    // this is kept track of and always matches the size of canvas. not sure
    // what sort of sys calls happen under the hood for SDL_GetWindowSize. this
    // is simpler.
    window_width: u32,
    window_height: u32,

    // sdl fundamental constructs. drop order is in stated order
    event_pump: sdl2::EventPump,
    canvas: sdl2::render::WindowCanvas,
    _sdl_video_subsystem: sdl2::VideoSubsystem,
    _sdl_context: sdl2::Sdl,
}

/// anything which is update in the game loop but not saved. intended for particle effects
pub trait Volatile {
    /// generate a change in self (rate), which is stored and applied later in this frame.\
    /// note: this takes a immutable reference to self, maybe store the rate in a Cell.\
    /// note: the rate should not be part of the save file, only the current state.
    fn generate_rate(&self, state: &GameState);
    
    /// apply the rate which was previously generated
    fn apply_rate(&mut self);

    /// draw to the screen
    /// window_size is the size of the canvas
    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32));

    /// false if should be removed from game loop
    fn alive(&self) -> bool { true }
}

/// anything updated in the game loop and saved as part of the save file
#[typetag::serde(tag = "type")]
pub trait Persistent: Volatile {}

impl GameState {
    /// intended to be used with save() and load().
    #[allow(dead_code)]
    fn gen_save_file_path(game_name: &'static str) -> String {
        let current_time = std::time::SystemTime::now();
        let duration_since_epoch = current_time.duration_since(std::time::UNIX_EPOCH).unwrap();
        let epoch_time_seconds = duration_since_epoch.as_secs();

        let mut pathbuf = dirs::data_local_dir().unwrap();
        pathbuf.push(game_name.to_lowercase().replace(" ", "_"));
        pathbuf.push("saves");
        pathbuf.push("");
        let path = pathbuf.to_str().unwrap();

        format!(
            "{}{}_{}",
            path,
            epoch_time_seconds.to_string(),
            std::process::id().to_string()
        )
    }

    pub fn new(win_title: &'static str, win_size: (u32, u32)) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let sdl_video_subsystem = sdl_context.video()?;
        let window = sdl_video_subsystem
            .window(win_title, win_size.0, win_size.1)
            .resizable()
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        let canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;
        let event_pump = sdl_context.event_pump()?;
        Ok(Self {
            persistent: Vec::new(),
            under_volatile: Vec::new(),
            over_volatile: Vec::new(),
            window_width: win_size.0,
            window_height: win_size.1,
            event_pump,
            canvas,
            _sdl_video_subsystem: sdl_video_subsystem,
            _sdl_context: sdl_context,
        })
    }

    /// overrides or creates new save file
    pub fn save(&mut self, save_file_path: String) -> Result<(), String> {
        let file = std::fs::File::create(save_file_path).map_err(|e| e.to_string())?;
        let mut writer = std::io::BufWriter::new(file);

        for elem in self.persistent.iter() {
            serde_json::to_writer(&mut writer, elem).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// reads save file and populates members
    pub fn load(&mut self, path: String) -> Result<(), String> {
        self.persistent.clear();
        self.under_volatile.clear();
        self.over_volatile.clear();

        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);
        self.persistent = serde_json::from_reader(reader)
            .map_err(|e| format!("Unable to deserialize file: {}", e))?;
        Ok(())
    }

    pub fn run(&mut self) {
        let seconds_per_frame = std::time::Duration::from_secs_f32(1f32 / 120f32);
        'outer: loop {
            let start = std::time::Instant::now();
            while let Some(event) = self.event_pump.poll_event() {
                match event {
                    sdl2::event::Event::Window {
                        timestamp: _,
                        window_id: _,
                        win_event,
                    } => match win_event {
                        sdl2::event::WindowEvent::SizeChanged(x_size, y_size) => {
                            self.window_width = x_size as u32;
                            self.window_height = y_size as u32;
                        }
                        _ => {}
                    },
                    sdl2::event::Event::Quit { .. }
                    | sdl2::event::Event::KeyDown {
                        keycode: Some(sdl2::keyboard::Keycode::Escape),
                        ..
                    } => break 'outer,
                    _ => {}
                }
            }

            // order is very important. accumulate rates from states
            self.persistent.iter().for_each(|c| c.generate_rate(&self));
            // apply rates to states
            self.persistent.iter_mut().for_each(|c| c.apply_rate());

            // generating and applying rates for particle effects happens after persistent
            // has acquired its new states
            self.under_volatile
                .iter()
                .chain(self.over_volatile.iter())
                .for_each(|c| c.generate_rate(&self));
            // apply rates to states
            self.under_volatile
                .iter_mut()
                .chain(self.over_volatile.iter_mut())
                .for_each(|c| c.apply_rate());

            // check removing particle effects first.
            // cleanup order important as a volatile element may check if a persistent is dead then remove itself
            self.under_volatile.retain(|e| e.alive());
            self.over_volatile.retain(|e| e.alive());
            // remove persistent after removing volatile.
            self.persistent.retain(|e| e.alive());

            // render all after update
            self.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
            self.canvas.clear();

            self.under_volatile
                .iter_mut()
                .for_each(|c| c.render(&mut self.canvas, (self.window_width, self.window_height)));
            self.persistent
                .iter_mut()
                .for_each(|c| c.render(&mut self.canvas, (self.window_width, self.window_height)));
            self.over_volatile
                .iter_mut()
                .for_each(|c| c.render(&mut self.canvas, (self.window_width, self.window_height)));
            self.canvas.present();

            let stop = std::time::Instant::now();
            let duration = stop - start;
            if duration < seconds_per_frame {
                std::thread::sleep(seconds_per_frame - duration);
            }
        }
    }
}
