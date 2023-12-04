use serde::ser::SerializeMap;

extern crate sdl2;

// anything which is part of the game loop which is not saved. e.g. particle effect
pub trait Volatile {
    /// generate a change in self (rate), which is stored and applied later in this frame.\
    /// note: this takes an immutable reference to self; maybe store the rate in a Cell.\
    /// note: the rate should not be part of the save file, only the current state.
    fn generate_rate(&self, state: &GameState);

    /// apply the rate which was previously generated.\
    /// first return value is false iff self should be removed from the game\
    /// second return value is a vector of elements to add to the game\
    /// and which layers they belong to\
    fn apply_rate(&mut self) -> (bool, Vec<(String, Vec<Entity>)>);

    /// draw to the screen
    /// window_size is the size of the canvas
    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32));
}

/// anything updated in the game loop and saved as part of the save file
#[typetag::serde(tag = "type")]
pub trait Persistent: Volatile {}

pub enum Entity {
    Volatile(Box<dyn Volatile>),
    Persistent(Box<dyn Persistent>),
}

impl Volatile for Entity {
    fn generate_rate(&self, state: &GameState) {
        match self {
            Entity::Volatile(v) => v.generate_rate(state),
            Entity::Persistent(p) => p.generate_rate(state),
        }
    }

    fn apply_rate(&mut self) -> (bool, Vec<(String, Vec<Entity>)>) {
        match self {
            Entity::Volatile(v) => v.apply_rate(),
            Entity::Persistent(p) => p.apply_rate(),
        }
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32)) {
        match self {
            Entity::Volatile(v) => v.render(canvas, window_size),
            Entity::Persistent(p) => p.render(canvas, window_size),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveState {
    // order in which layers are rendered
    pub layer_name_order: Vec<String>,

    // all things which are part of the game loop. associates layer name with
    // entities in that layer
    #[serde(
        serialize_with = "serialize_save_state_layers",
        deserialize_with = "deserialize_save_state_layers"
    )]
    pub layers: std::collections::BTreeMap<String, Vec<Entity>>,
}

fn serialize_save_state_layers<S>(
    layers: &std::collections::BTreeMap<String, Vec<Entity>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut state = serializer.serialize_map(Some(layers.len()))?;
    for (key, entities) in layers {
        let persistent_entities: Vec<&dyn Persistent> = entities
            .iter()
            .filter_map(|e| match e {
                Entity::Volatile(_) => None,
                Entity::Persistent(p) => Some(p.as_ref()),
            })
            .collect();
        state.serialize_entry(key, &persistent_entities)?;
    }
    state.end()
}

fn deserialize_save_state_layers<'de, D>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, Vec<Entity>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SaveStateLayersVisitor;
    impl<'de> serde::de::Visitor<'de> for SaveStateLayersVisitor {
        type Value = std::collections::BTreeMap<String, Vec<Entity>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map with render layer keys, and values for persistent entities within those layers")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut layers = std::collections::BTreeMap::new();

            while let Some((key, persistent_entities)) = map.next_entry::<String, Vec<Box<dyn Persistent>>>()? {
                let entities = persistent_entities.into_iter().map(|p| Entity::Persistent(p)).collect();
                layers.insert(key, entities);
            }
            Ok(layers)
        }
    }

    deserializer.deserialize_map(SaveStateLayersVisitor)
}

impl SaveState {
    pub fn clear(&mut self) {
        self.layer_name_order.clear();
        self.layers.clear();
    }

    pub fn new(layer_name_order: Vec<String>) -> Self {
        let layers: std::collections::BTreeMap<String, Vec<Entity>> = layer_name_order
            .iter()
            .map(|key| (key.to_owned(), Vec::new()))
            .collect();
        Self {
            layer_name_order,
            layers,
        }
    }
}

pub struct GameState {
    pub save_state: SaveState,

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
        Ok(Self {
            save_state: SaveState::new(layer_name_order),
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
        serde_json::to_writer(&mut writer, &self.save_state).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// reads save file and populates members
    pub fn load(&mut self, path: String) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);
        self.save_state = serde_json::from_reader(reader).map_err(|e| e.to_string())?;
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

            self.save_state
                .layers
                .values()
                .for_each(|layer| layer.iter().for_each(|c| c.generate_rate(&self)));

            let mut spawned: Vec<(String, Vec<Entity>)> = Vec::new();

            for layer_name in self.save_state.layer_name_order.iter() {
                match self.save_state.layers.get_mut(layer_name) {
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

            self.save_state.layers.values().for_each(|layer| {
                layer.iter().for_each(|c| {
                    c.render(&mut self.canvas, (self.window_width, self.window_height))
                })
            });

            self.canvas.present();

            // insert the spawned elements after the new states are available
            for mut s in spawned {
                let layer = self
                    .save_state
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
