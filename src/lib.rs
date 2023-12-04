extern crate sdl2;

pub struct GameState {
    // order in which layers are rendered
    pub layer_name_order: Vec<String>,

    // all things which are part of the game loop. the name with the things in that layer
    pub layers: std::collections::BTreeMap<String, Vec<Box<dyn Volatile>>>,

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
    /// note: this takes an immutable reference to self; maybe store the rate in a Cell.\
    /// note: the rate should not be part of the save file, only the current state.
    fn generate_rate(&self, state: &GameState);

    /// apply the rate which was previously generated.\
    /// first return value is false iff self should be removed from the game\
    /// second return value is a vector of elements to add to the game\
    /// and which layers they belong to\
    fn apply_rate(&mut self) -> (bool, Vec<(String, Vec<Box<dyn Volatile>>)>);

    /// draw to the screen
    /// window_size is the size of the canvas
    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32));

    // allow downcast to Persistent
    fn as_any(&self) -> &dyn std::any::Any;
}

/// anything updated in the game loop and saved as part of the save file
#[typetag::serde(tag = "type")]
pub trait Persistent : Volatile {}

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

    /// create a game state, with associated window and sdl context. \
    /// `win_title` and `win_size` are used to set the properties of the window \
    /// `layer_name_order` is the set of layer names to register; used internally to \
    /// determine rendering order of sprites
    pub fn new(
        win_title: &'static str,
        win_size: (u32, u32),
        layer_name_order: Vec<String>,
    ) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let sdl_video_subsystem = sdl_context.video()?;
        let window = sdl_video_subsystem
            .window(win_title, win_size.0, win_size.1)
            .resizable()
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;
        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        let event_pump = sdl_context.event_pump()?;

        let tree: std::collections::BTreeMap<String, Vec<Box<dyn Volatile>>> = layer_name_order
            .iter()
            .map(|key| (key.to_owned(), Vec::new()))
            .collect();
        Ok(Self {
            layer_name_order,
            layers: tree,
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

        serde_json::to_writer(&mut writer, &self.layer_name_order).map_err(|e| e.to_string())?;

        for name_layer in self.layers.iter() {
            let s = name_layer.0;
            serde_json::to_writer(&mut writer, s).map_err(|e| e.to_string())?;
            for elem in name_layer.1 {
                if let Ok(persistent_ref) = elem.as_any()

                }

                let as_persistent = match elem.as_any().downcast_ref::<&dyn Persistent>() {
                    Some(p) => p,
                    None => continue,
                };
                println!("one");
                serde_json::to_writer(&mut writer, as_persistent).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// reads save file and populates members
    pub fn load(&mut self, path: String) -> Result<(), String> {
        self.layer_name_order.clear();
        self.layers.clear();

        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);

        let deserializer = serde_json::Deserializer::from_reader(reader);
        let iterator = deserializer.into_iter::<serde_json::Value>();
        for item in iterator {
            println!("Got {:?}", item.unwrap());
        }
        // todo

        // self.layer_name_order = serde_json::from_reader(reader).map_err(|e| e.to_string())?;
        // while (reader.)
        // let key: String = serde_json::from_reader(reader).map_err(|e| e.to_string())?;

        // self.persistent = serde_json::from_reader(reader)
        // .map_err(|e| format!("Unable to deserialize file: {}", e))?;
        Ok(())
    }

    /// f is a closure that handles sdl2 events. returns false iff run should return
    pub fn run<F>(&mut self, f: F)
    where
        F: Fn(sdl2::event::Event) -> bool,
    {
        let seconds_per_frame = std::time::Duration::from_secs_f32(1f32 / 120f32);
        'outer: loop {
            let start = std::time::Instant::now();
            while let Some(event) = self.event_pump.poll_event() {
                // detect for window size change
                if let sdl2::event::Event::Window {
                    timestamp: _,
                    window_id: _,
                    win_event,
                } = event
                {
                    if let sdl2::event::WindowEvent::SizeChanged(x_size, y_size) = win_event {
                        self.window_width = x_size as u32;
                        self.window_height = y_size as u32;
                    }
                }
                // forward all event to the closure
                if !f(event) {
                    break 'outer;
                }
            }

            self.layers
                .values()
                .for_each(|layer| layer.iter().for_each(|c| c.generate_rate(&self)));

            let mut spawned: Vec<(String, Vec<Box<dyn Volatile>>)> = Vec::new();

            for layer_name in self.layer_name_order.iter() {
                match self.layers.get_mut(layer_name) {
                    Some(layer) => {
                        let len = layer.len();
                        for i in (0..len).rev() {
                            let elem = &mut layer[i];
                            let mut val = elem.apply_rate();
                            if !val.0 {
                                layer.remove(i);
                            }
                            spawned.append(&mut val.1);
                        }
                    }
                    None => {}
                }
            }
            self.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
            self.canvas.clear();

            self.layers.values().for_each(|layer| {
                layer.iter().for_each(|c| {
                    c.render(&mut self.canvas, (self.window_width, self.window_height))
                })
            });

            self.canvas.present();

            // insert the spawned elements after the new states are available
            for mut s in spawned {
                let layer = self
                    .layers
                    .get_mut(&s.0)
                    .expect(&format!("Tried spawning to unregisted layer: {}", &s.0));
                layer.append(&mut s.1);
            }

            let stop = std::time::Instant::now();
            let duration = stop - start;
            if duration < seconds_per_frame {
                std::thread::sleep(seconds_per_frame - duration);
            }
        }
    }
}
