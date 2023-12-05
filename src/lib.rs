use serde::ser::SerializeMap;
use std::cell::Cell;

extern crate sdl2;

// anything which is part of the game loop which is not saved. e.g. particle effect
pub trait Volatile {
    /// generate a change in self (rate), which is stored and applied later in this frame.\
    /// note: the rate should not be part of the save file, only the current state.
    fn generate_rate(&mut self, state: &GameState);

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
    // note that generate_rate takes a mutable ref to self and a immutable ref
    // to state. state includes all entities, including this one which is having
    // generate_rate called on it. so we have a mutable reference to self and a
    // immutable reference to self via state.layer_state.layers[i]. to satisfy
    // borrow checking rules, the entity which is generating its rate is taken,
    // leaving Taken as a placeholder value. self if now the only reference,
    // satsifying the borrow checker.
    Taken,
    Volatile(Box<dyn Volatile>),
    Persistent(Box<dyn Persistent>),
}

impl Default for Entity {
    fn default() -> Self {
        Entity::Taken
    }
}

impl Volatile for Entity {
    fn generate_rate(&mut self, state: &GameState) {
        match self {
            Entity::Volatile(v) => v.generate_rate(state),
            Entity::Persistent(p) => p.generate_rate(state),
            Entity::Taken => panic!("generate_rate on Taken"),
        }
    }

    fn apply_rate(&mut self) -> (bool, Vec<(String, Vec<Entity>)>) {
        match self {
            Entity::Volatile(v) => v.apply_rate(),
            Entity::Persistent(p) => p.apply_rate(),
            Entity::Taken => panic!("apply_rate on Taken"),
        }
    }

    fn render(&self, canvas: &mut sdl2::render::WindowCanvas, window_size: (u32, u32)) {
        match self {
            Entity::Volatile(v) => v.render(canvas, window_size),
            Entity::Persistent(p) => p.render(canvas, window_size),
            Entity::Taken => panic!("render on Taken"),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LayersWrapper {
    // all things which are part of the game loop. associates layer name with
    // entities in that layer
    #[serde(
        serialize_with = "LayersWrapper::serialize_layers",
        deserialize_with = "LayersWrapper::deserialize_layers"
    )]
    pub layers: std::collections::BTreeMap<String, Vec<Cell<Entity>>>,
}

impl LayersWrapper {
    fn new(layer_names: &'static [&'static str]) -> Self {
        let layers: std::collections::BTreeMap<String, Vec<Cell<Entity>>> = layer_names
            .iter()
            .map(|key| ((*key).to_owned(), Vec::new()))
            .collect();
        Self { layers }
    }

    fn serialize_layers<S>(
        layers: &std::collections::BTreeMap<String, Vec<Cell<Entity>>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(layers.len()))?;
        for (key, entities) in layers {
            // takes all persistent entities from the layer and serializes them,
            // then return them back into their cells
            let mut persistent_entities: Vec<Box<dyn Persistent>> = Vec::new();
            let mut persistent_entities_return: Vec<&Cell<Entity>> = Vec::new();

            for entity_cell in entities.iter() {
                let entity = entity_cell.take();
                match entity {
                    Entity::Volatile(_) => {
                        entity_cell.set(entity); // return immediately
                    }
                    Entity::Persistent(p) => {
                        persistent_entities.push(p);
                        persistent_entities_return.push(entity_cell);
                    }
                    Entity::Taken => panic!("serialize on Taken"),
                }
            }

            state.serialize_entry(key, &persistent_entities)?;

            for (persistent_entity, cell) in persistent_entities
                .into_iter()
                .zip(persistent_entities_return)
            {
                let entity = Entity::Persistent(persistent_entity);
                cell.set(entity);
            }
        }
        state.end()
    }

    fn deserialize_layers<'de, D>(
        deserializer: D,
    ) -> Result<std::collections::BTreeMap<String, Vec<Cell<Entity>>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LayerStateLayersVisitor;
        impl<'de> serde::de::Visitor<'de> for LayerStateLayersVisitor {
            type Value = std::collections::BTreeMap<String, Vec<Cell<Entity>>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with render layer keys, and values for persistent entities within those layers")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut layers = std::collections::BTreeMap::new();

                while let Some((key, persistent_entities)) =
                    map.next_entry::<String, Vec<Box<dyn Persistent>>>()?
                {
                    let entities = persistent_entities
                        .into_iter()
                        .map(|p| Cell::new(Entity::Persistent(p)))
                        .collect();
                    layers.insert(key, entities);
                }
                Ok(layers)
            }
        }

        deserializer.deserialize_map(LayerStateLayersVisitor)
    }
}

pub struct GameState {
    // render order of layers
    layer_names: &'static [&'static str],

    // all things which are part of the game loop. associates layer name with
    // entities in that layer
    layer_wrapper: LayersWrapper,

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
    /// `win_title` and `win_size` are used to set the properties of the window\
    /// `layer_names` is the set of layer names to register; used to indicate
    /// RENDERING ORDER of sprites
    pub fn new(
        win_title: &'static str,
        win_size: (u32, u32),
        layer_names: &'static [&'static str],
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

        let layers = LayersWrapper::new(layer_names);

        Ok(Self {
            layer_names,
            layer_wrapper: layers,
            window_width: win_size.0,
            window_height: win_size.1,
            event_pump,
            canvas,
            _sdl_video_subsystem: sdl_video_subsystem,
            _sdl_context: sdl_context,
        })
    }

    /// overrides or creates new save file
    pub fn save(&self, save_file_path: String) -> Result<(), String> {
        let file = std::fs::File::create(save_file_path).map_err(|e| e.to_string())?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self.layer_wrapper).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// reads save file and populates members
    pub fn load(&mut self, path: String) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);
        let incoming_layer_wrapper: LayersWrapper = serde_json::from_reader(reader).map_err(|e| e.to_string())?;
        if ! incoming_layer_wrapper.layers.keys().eq(self.layer_wrapper.layers.keys()) {
            return Err("loaded save file doesn't contain correct render layers".to_owned());
        }
        self.layer_wrapper = incoming_layer_wrapper;
        Ok(())
    }

    pub fn clear_entities(&mut self) {
        self.layer_wrapper.layers.values_mut().for_each(|v| v.clear());
    }

    pub fn spawn(&mut self, e: Entity, layer: String) {
        self.layer_wrapper
            .layers
            .get_mut(&layer)
            .expect(&format!("Manual spawn to unregistered layer: {}", layer))
            .push(Cell::new(e));
    }

    /// f is a closure that handles sdl2 events. returns false of err iff run
    /// should return
    pub fn run<F>(&mut self, f: F) -> Result<(), String>
    where
        F: Fn(&mut Self, sdl2::event::Event) -> Result<bool, String>,
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
                match f(self, event) {
                    Ok(alive) => {
                        if !alive {
                            break 'outer; // closure requested finish
                        }
                    }
                    Err(e) => return Err(e), // propagate error
                };
            }

            // generate rates
            self.layer_wrapper.layers.values().for_each(|layer| {
                layer.iter().for_each(|entity_cell| {
                    let mut entity = entity_cell.take();
                    entity.generate_rate(&self);
                    entity_cell.set(entity);
                })
            });

            let mut spawned: Vec<(String, Vec<Entity>)> = Vec::new();

            // apply rate - handle despawn
            self.layer_wrapper.layers.values_mut().for_each(|layer| {
                let len = layer.len();
                for i in (0..len).rev() {
                    let entity_cell = &layer[i];
                    let mut entity = entity_cell.take();
                    let mut val = entity.apply_rate();
                    entity_cell.set(entity);
                    if !val.0 {
                        layer.remove(i);
                    }
                    spawned.append(&mut val.1);
                }
            });

            self.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
            self.canvas.clear();

            // insert spawned elements after the new states are available
            for s in spawned {
                let layer = self.layer_wrapper.layers.get_mut(&s.0).expect(&format!(
                    "Entity created spawn for unregistered layer: {}",
                    &s.0
                ));

                let mut spawned_as_cells = s.1.into_iter().map(Cell::new).collect();
                layer.append(&mut spawned_as_cells);
            }

            // render all
            self.layer_names.iter().for_each(|layer_name| {
                self.layer_wrapper.layers.get(*layer_name).unwrap().iter().for_each(|entity_cell| {
                    let entity = entity_cell.take();
                    entity.render(&mut self.canvas, (self.window_width, self.window_height));
                    entity_cell.set(entity);
                })
            });

            self.canvas.present();

            let stop = std::time::Instant::now();
            let duration = stop - start;
            if duration < seconds_per_frame {
                std::thread::sleep(seconds_per_frame - duration);
            }
        }
        Ok(())
    }
}
