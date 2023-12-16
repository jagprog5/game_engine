use core::panic;
use downcast_rs::{impl_downcast, Downcast};
use sdl2::render::WindowCanvas;
use serde::ser::{SerializeMap, SerializeStruct};
use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
    rc::Weak,
};

extern crate sdl2;

/// signaling for entity despawn
pub enum LivelinessStatus {
    Retain,
    Despawn,
}

impl LivelinessStatus {
    pub fn new(b: bool) -> Self {
        if b { LivelinessStatus::Retain } else { LivelinessStatus::Despawn }
    }
}

pub struct PersistentSpawnChanges {
    pub alive: LivelinessStatus,
    /// the spawns into the game, and the render layers they are added to.
    pub volatile_spawns: Vec<(&'static str, Vec<VolatileSpawn>)>,
    /// same as volatile_spawns, but for PersistentSpawn instead
    pub persistent_spawns: Vec<(&'static str, Vec<PersistentSpawn>)>,
}

pub struct VolatileSpawnChanges {
    pub alive: LivelinessStatus,
    pub volatile_spawns: Vec<(&'static str, Vec<VolatileSpawn>)>,
}

/// anything which is part of the game loop and is not saved. e.g. particle effect.
pub trait Volatile: Downcast {
    /// second thing to happen per frame (preceded by sdl event handling)\
    /// generate a change in self (rate), which is stored and applied later in this frame.\
    /// note: the rate should not be part of the save file, only the current state.
    fn generate_rate(&mut self, state: &GameState);

    /// third thing to happen per frame\
    /// apply the rate which was previously generated
    fn apply_rate(&mut self);

    /// fourth thing to happen per frame\
    fn apply_spawns(&self) -> VolatileSpawnChanges {
        // default impl is spawns nothing and alive forever
        VolatileSpawnChanges {
            alive: LivelinessStatus::Retain,
            volatile_spawns: Vec::new(),
        }
    }

    /// last thing to happen per frame\
    /// draw to the screen\
    fn render(&self, canvas: &mut WindowCanvas);
}
impl_downcast!(Volatile);

/// anything updated in the game loop and saved as part of the save file
#[typetag::serde(tag = "type")]
pub trait Persistent: Downcast {
    /// first thing to happen per frame\
    /// generate a change in self (rate), which is stored and applied later in this frame.\
    /// note: the rate should not be part of the save file, only the current state.
    fn generate_rate(&mut self, state: &GameState);

    /// second thing to happen per frame\
    /// apply the rate which was previously generated
    fn apply_rate(&mut self);

    /// third thing to happen per frame\
    fn apply_spawns(&self) -> PersistentSpawnChanges {
        // default impl is spawns nothing and alive forever
        PersistentSpawnChanges {
            alive: LivelinessStatus::Retain,
            volatile_spawns: Vec::new(),
            persistent_spawns: Vec::new(),
        }
    }

    /// last thing to happen per frame\
    /// draw to the screen\
    fn render(&self, canvas: &mut WindowCanvas);

    /// references to Persistent objects which need to be saved
    fn save_entity_references(&self) -> Vec<MaybePersistentRef> {
        Vec::new()
    }

    /// same number of elements is given here as was returned by save_entity_references
    fn load_entity_references(&mut self, v: Vec<MaybePersistentRef>) {
        if !v.is_empty() {
            panic!("persistent entity references provided to instance that doesn't take any");
        }
    }
}
impl_downcast!(Persistent);

// Rc: there are strong references from game state to each entity,
// and weak reference between entities
// Cell, Option: provides interior mutability for each instance.
// Box: since the size is not known at compile time.
pub struct VolatileEntity(pub Rc<Cell<Option<Box<dyn Volatile>>>>);

// functions forward to Volatile
impl VolatileEntity {
    fn generate_rate(&self, state: &GameState) {
        let mut e = self.0.take().unwrap();
        e.generate_rate(state);
        self.0.set(Some(e));
    }

    fn apply_rate(&self) {
        let mut e = self.0.take().unwrap();
        e.apply_rate();
        self.0.set(Some(e));
    }

    fn apply_spawns(&self) -> VolatileSpawnChanges {
        let e = self.0.take().unwrap();
        let r = e.apply_spawns();
        self.0.set(Some(e));
        r
    }

    fn render(&self, canvas: &mut WindowCanvas) {
        let e = self.0.take().unwrap();
        e.render(canvas);
        self.0.set(Some(e));
    }
}

// shared pointer to persistent
pub struct PersistentEntity(pub Rc<Cell<Option<Box<dyn Persistent>>>>);

// functions forward to Persistent
impl PersistentEntity {
    fn clone(&self) -> Self {
        PersistentEntity(self.0.clone())
    }

    fn generate_rate(&self, state: &GameState) {
        let mut e = self.0.take().unwrap();
        e.generate_rate(state);
        self.0.set(Some(e));
    }

    fn apply_rate(&self) {
        let mut e = self.0.take().unwrap();
        e.apply_rate();
        self.0.set(Some(e));
    }

    fn apply_spawns(&self) -> PersistentSpawnChanges {
        let e = self.0.take().unwrap();
        let r = e.apply_spawns();
        self.0.set(Some(e));
        r
    }

    fn render(&self, canvas: &mut WindowCanvas) {
        let e = self.0.take().unwrap();
        e.render(canvas);
        self.0.set(Some(e));
    }

    fn save_entity_references(&self) -> Vec<MaybePersistentRef> {
        let e = self.0.take().unwrap();
        let r = e.save_entity_references();
        self.0.set(Some(e));
        r
    }

    fn load_entity_references(&self, v: Vec<MaybePersistentRef>) {
        let mut e = self.0.take().unwrap();
        e.load_entity_references(v);
        self.0.set(Some(e));
    }
}

// hash and equality operators based on pointer address for use in unordered set
// and unordered map when loading and saving
impl PartialEq for PersistentEntity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PersistentEntity {}

impl std::hash::Hash for PersistentEntity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = &*self.0 as *const _ as usize;
        ptr.hash(state);
    }
}

pub struct PersistentRef(pub Weak<Cell<Option<Box<dyn Persistent>>>>);

impl Clone for PersistentRef {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// a weak reference between two persistent entities, which will be saved in the
/// save file and restored on load
pub enum MaybePersistentRef {
    /// None is the default state for a MaybePersistentRef, indicating it hasn't been set yet
    None,
    /// indicates that on save this ref pointed to something which is no longer
    /// part of the game.
    Despawned,
    Some(PersistentRef),
}

impl Default for MaybePersistentRef {
    fn default() -> Self {
        MaybePersistentRef::None
    }
}

impl Clone for MaybePersistentRef {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Despawned => Self::Despawned,
            Self::Some(arg0) => Self::Some(arg0.clone()),
        }
    }
}

/// result of promoting a PersistentRef to a PersistentEntity
pub enum PersistentRefPromotionResult {
    /// promoting the weak reference failed because it despawned.\
    /// typically this should be handled the same as for `MaybePersistentRef::Despawned`
    Despawned,
    // someone else is looking at this reference, most likely because of a self
    // reference. or from improper api use when a PersistentRef wasn't returned
    Taken,
    // 1 is an exclusive reference to the entity.
    // 0 is the place that it came from. it gets push back by `PersistentRef::set`
    Some(PersistentEntity, Box<dyn Persistent>),
}

impl PersistentRef {
    // if Some is returned, it must be followed by a corresponding call to set
    pub fn get(&self) -> PersistentRefPromotionResult {
        let rc = match self.0.upgrade() {
            Some(rc) => rc,
            None => return PersistentRefPromotionResult::Despawned,
        };

        match rc.take() {
            Some(e) => PersistentRefPromotionResult::Some(PersistentEntity(rc), e),
            None => PersistentRefPromotionResult::Taken,
        }
    }

    // return the PersistentEntity back to its position
    pub fn set(s: (PersistentEntity, Box<dyn Persistent>)) {
        s.0 .0.set(Some(s.1));
    }
}

pub type PersistentSpawn = Box<dyn Persistent>;
pub type VolatileRef = Weak<Cell<Option<Box<dyn Volatile>>>>;
pub type VolatileSpawn = Box<dyn Volatile>;

#[derive(serde::Serialize, serde::Deserialize)]
/// corresponds to variants of `MaybePersistentRef`
enum Tag {
    None,
    Despawned,
    Some(u64),
}

// used internally for PersistentState serialize and deserialize
struct TaggedPersistent {
    // the subject persistent entity. it takes a clone of the Rc the game state has
    e: PersistentEntity,
    // the id for this entity
    tag: u64,
    // the tags that this entity points to
    refs: Vec<Tag>,
}

impl serde::Serialize for TaggedPersistent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TaggedPersistent", 3)?;
        let e = self.e.0.take().unwrap();
        state.serialize_field("e", &e)?;
        self.e.0.set(Some(e));
        state.serialize_field("tag", &self.tag)?;
        state.serialize_field("refs", &self.refs)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for TaggedPersistent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct TaggedPersistentHelper {
            e: Box<dyn Persistent>,
            tag: u64,
            refs: Vec<Tag>,
        }

        let helper: TaggedPersistentHelper = serde::de::Deserialize::deserialize(deserializer)?;
        Ok(TaggedPersistent {
            e: PersistentEntity(Rc::new(Cell::new(Some(helper.e)))),
            tag: helper.tag,
            refs: helper.refs,
        })
    }
}

/// section of GameState that has saveable things
#[derive(serde::Serialize)]
struct PersistentState {
    /// associates layer name with persistent entities in that layer
    #[serde(
        serialize_with = "PersistentState::serialize_layers",
        deserialize_with = "PersistentState::deserialize_layers"
    )]
    pub persistent_layers: BTreeMap<&'static str, Vec<PersistentEntity>>,
}

impl PersistentState {
    fn new(layer_names: &'static [&'static str]) -> Self {
        let persistent_layers: BTreeMap<&'static str, Vec<PersistentEntity>> =
            layer_names.iter().map(|key| (*key, Vec::new())).collect();
        Self { persistent_layers }
    }

    // saving has linear time complexity with the number of elements
    fn serialize_layers<S>(
        layers: &BTreeMap<&'static str, Vec<PersistentEntity>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(layers.len()))?;
        let mut next_tag: u64 = 0;

        let mut tagged_entities: BTreeMap<&'static str, Vec<TaggedPersistent>> = BTreeMap::new();

        // associates entities with their tags. for time complexity later
        let mut lookup_tag: HashMap<PersistentEntity, u64> = HashMap::new();

        // for all entities in the layers get a rc clone (serde requires data to
        // be owned), and set a tag uniquely identifying PersistentEntities
        for (key, entities) in layers {
            let mut tagged_entities_in_layer: Vec<TaggedPersistent> = Vec::new();
            for entity in entities.iter() {
                // create the lookup association, but don't bother if it's
                // guarenteed that this association will not be used
                if Rc::weak_count(&entity.0) != 0 {
                    // this if statement errs on the side of caution:
                    // it's possible that the weak count will not be zero from a
                    // Volatile looking at this, in which case it will create a
                    // association that's not used (only persistent ->
                    // persistent weak ref is saved and loaded), but that's
                    // fine. this check doesn't really matter anyway
                    lookup_tag.insert(entity.clone(), next_tag);
                }

                tagged_entities_in_layer.push(TaggedPersistent {
                    e: entity.clone(),
                    tag: next_tag,
                    refs: Vec::new(), // populated in next step
                });
                next_tag += 1;
            }

            tagged_entities.insert(key, tagged_entities_in_layer);
        }

        // next pass sets the refs to their tags
        for (_, tagged_entities_in_layer) in tagged_entities.iter_mut() {
            for tagged_entity in tagged_entities_in_layer.iter_mut() {
                for maybe_weak in tagged_entity.e.save_entity_references() {
                    match maybe_weak {
                        MaybePersistentRef::None => tagged_entity.refs.push(Tag::None),
                        MaybePersistentRef::Despawned => tagged_entity.refs.push(Tag::Despawned),
                        MaybePersistentRef::Some(weak) => {
                            let strong = weak.0.upgrade();
                            if strong.is_none() {
                                tagged_entity.refs.push(Tag::Despawned);
                            } else {
                                let p = PersistentEntity(strong.unwrap());
                                let tag = lookup_tag.get(&p).unwrap();
                                tagged_entity.refs.push(Tag::Some(*tag));
                            }
                        }
                    }
                }
            }
        }

        // do serialization
        for (k, v) in tagged_entities.into_iter() {
            state.serialize_entry(&k, &v)?;
        }

        state.end()
    }
}

// this is a PersistentState, but a temporary during deserialization. it uses String instead of str
#[derive(serde::Deserialize)]
struct PersistentStateTemp {
    #[serde(deserialize_with = "PersistentStateTemp::deserialize_layers")]
    pub persistent_layers: BTreeMap<String, Vec<PersistentEntity>>,
}

macro_rules! debug_assert_layers_rc_sanity {
    ($layers:expr) => {
        debug_assert!(
            {
                let mut good = true;
                $layers.values().for_each(|layer| {
                    layer.iter().for_each(|entity| {
                        if Rc::strong_count(&entity.0) != 1 {
                            good = false;
                        }
                    })
                });
                good
            },
            "only the game state is allowed strong references to entities. \
            inter-entity references should be weak. this possibly leaks"
        );
    };
}

impl PersistentStateTemp {
    fn replace(&mut self, state: &mut PersistentState) -> Result<(), String> {
        if !self
            .persistent_layers
            .keys()
            .eq(state.persistent_layers.keys())
        {
            return Err("loaded save file doesn't contain correct render layers".to_owned());
        }

        debug_assert_layers_rc_sanity!(&state.persistent_layers);
        for (k, to) in state.persistent_layers.iter_mut() {
            let from = self.persistent_layers.get_mut(k.to_owned()).unwrap();
            std::mem::swap(to, from);
        }
        Ok(())
    }

    // loading has same time complexity as saving
    fn deserialize_layers<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<String, Vec<PersistentEntity>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = BTreeMap<String, Vec<TaggedPersistent>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("map with schema specific for persistent entities")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut layers = BTreeMap::new();
                while let Some((key, tagged_entities_in_layer)) = map.next_entry()? {
                    layers.insert(key, tagged_entities_in_layer);
                }
                Ok(layers)
            }
        }

        let tagged_entities = deserializer.deserialize_map(Visitor)?;

        // all persistent entities have tags. just look at which ones are used
        let mut referenced_tags: HashSet<u64> = HashSet::new();
        for (_layer, tagged_entities_in_layer) in tagged_entities.iter() {
            for tagged_entity in tagged_entities_in_layer.iter() {
                for maybe_tag in tagged_entity.refs.iter() {
                    if let Tag::Some(tag) = maybe_tag {
                        referenced_tags.insert(*tag);
                    }
                }
            }
        }

        let mut lookup_entity: HashMap<u64, PersistentEntity> = HashMap::new();

        // associates tags with the entities. for time complexity later
        for (_layer, tagged_entities_in_layer) in tagged_entities.iter() {
            for tagged_entity in tagged_entities_in_layer.iter() {
                if referenced_tags.get(&tagged_entity.tag).is_some() {
                    lookup_entity.insert(tagged_entity.tag, tagged_entity.e.clone());
                }
            }
        }

        // recreate the references based on the tags
        let mut layers: BTreeMap<String, Vec<PersistentEntity>> = BTreeMap::new();

        for (layer, tagged_entities_in_layer) in tagged_entities.into_iter() {
            let mut entities_in_layer: Vec<PersistentEntity> = Vec::new();
            for tagged_entity in tagged_entities_in_layer.into_iter() {
                let p = tagged_entity.e;
                let refs: Vec<MaybePersistentRef> = tagged_entity
                    .refs
                    .iter()
                    .map(|r| match r {
                        Tag::Some(u) => {
                            let r = Rc::downgrade(&lookup_entity.get(u).unwrap().0);
                            MaybePersistentRef::Some(PersistentRef(r))
                        }
                        Tag::Despawned => MaybePersistentRef::Despawned,
                        Tag::None => MaybePersistentRef::None,
                    })
                    .collect();
                p.load_entity_references(refs);
                entities_in_layer.push(p);
            }
            layers.insert(layer.to_owned(), entities_in_layer);
        }

        Ok(layers)
    }
}

pub struct GameState {
    /// render order of layers
    layer_names: &'static [&'static str],

    persistent_state: PersistentState,

    /// associates layer name with volatile entities in that layer
    volatile_layers: BTreeMap<&'static str, Vec<VolatileEntity>>,

    // sdl fundamentals. drop order is in stated order
    event_pump: sdl2::EventPump,
    pub canvas: WindowCanvas,
    _sdl_video_subsystem: sdl2::VideoSubsystem,
    _sdl_context: sdl2::Sdl,
}

impl Drop for GameState {
    fn drop(&mut self) {
        debug_assert_layers_rc_sanity!(&self.persistent_state.persistent_layers);
        debug_assert_layers_rc_sanity!(&self.volatile_layers);
    }
}

impl GameState {
    pub const GOAL_FPS: f32 = 120f32;

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
        let persistent_state = PersistentState::new(layer_names);
        let volatile_layers: BTreeMap<&'static str, Vec<VolatileEntity>> =
            layer_names.iter().map(|key| (*key, Vec::new())).collect();

        Ok(Self {
            layer_names,
            persistent_state,
            volatile_layers,
            event_pump,
            canvas,
            _sdl_video_subsystem: sdl_video_subsystem,
            _sdl_context: sdl_context,
        })
    }

    /// overrides or creates new save file for the persistent entities
    pub fn save(&self, save_file_path: String) -> Result<(), String> {
        let file = std::fs::File::create(save_file_path).map_err(|e| e.to_string())?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self.persistent_state).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// reads save file and replaces only persistent entities member\
    /// consider first calling clear to also remove volatile entities
    pub fn load(&mut self, path: String) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);
        let mut incoming_persistent_state: PersistentStateTemp =
            serde_json::from_reader(reader).map_err(|e| e.to_string())?;
        incoming_persistent_state.replace(&mut self.persistent_state)?;
        Ok(())
    }

    /// clear only persistent entities
    pub fn clear_persistent(&mut self) {
        debug_assert_layers_rc_sanity!(&self.persistent_state.persistent_layers);
        self.persistent_state
            .persistent_layers
            .values_mut()
            .for_each(|v| v.clear());
    }

    /// clears all entities
    pub fn clear(&mut self) {
        self.clear_persistent();
        debug_assert_layers_rc_sanity!(&self.volatile_layers);
        self.volatile_layers.values_mut().for_each(|v| v.clear());
    }

    /// spawn a volatile entity to a render layer
    pub fn spawn_volatile(&mut self, e: VolatileSpawn, layer: &'static str) {
        self.volatile_layers
            .get_mut(&layer)
            .expect(&format!(
                "Spawn of volatile to unregistered layer: {}",
                layer
            ))
            .push(VolatileEntity(Rc::new(Cell::new(Some(e)))));
    }

    /// spawn a persistent entity to a render layer
    pub fn spawn_persistent(&mut self, e: PersistentSpawn, layer: &'static str) {
        self.persistent_state
            .persistent_layers
            .get_mut(&layer)
            .expect(&format!(
                "Spawn of persistent to unregistered layer: {}",
                layer
            ))
            .push(PersistentEntity(Rc::new(Cell::new(Some(e)))));
    }

    pub fn get_volatiles(&self, layer: &'static str) -> &Vec<VolatileEntity> {
        self.volatile_layers
            .get(layer)
            .expect(&format!("get_volatiles on unregistered layer: {}", layer))
    }

    pub fn get_persistents(&self, layer: &'static str) -> &Vec<PersistentEntity> {
        self.persistent_state
            .persistent_layers
            .get(layer)
            .expect(&format!("get_persistents on unregistered layer: {}", layer))
    }

    /// event_handler closure should return false (dead) or err only if run should return. it handles sdl2 events\
    /// post_render_hook is a render function over top of the game. it may return an error string which also causes run to return\
    pub fn run<EventHandler,PostRenderHook>(&mut self, event_handler: EventHandler, post_render_hook: PostRenderHook) -> Result<(), String>
    where
    EventHandler: Fn(&mut Self, &sdl2::event::Event) -> Result<bool, String>,
    PostRenderHook: Fn(&mut WindowCanvas)
    {
        let seconds_per_frame = std::time::Duration::from_secs_f32(1f32 / Self::GOAL_FPS);
        'outer: loop {
            let start = std::time::Instant::now();
            while let Some(event) = self.event_pump.poll_event() {
                // forward all event to the closure
                match event_handler(self, &event) {
                    Ok(alive) => {
                        if !alive {
                            break 'outer; // closure requested finish
                        }
                    }
                    Err(e) => return Err(e), // propagate error
                };
            }

            // generate rates
            self.persistent_state
                .persistent_layers
                .values()
                .for_each(|entities| {
                    entities.iter().for_each(|entity| {
                        entity.generate_rate(&self);
                    })
                });
            self.volatile_layers.values().for_each(|entities| {
                entities.iter().for_each(|entity| {
                    entity.generate_rate(&self);
                })
            });

            // apply rates
            self.persistent_state
                .persistent_layers
                .values()
                .for_each(|entities| {
                    entities.iter().for_each(|entity| {
                        entity.apply_rate();
                    })
                });
            self.volatile_layers.values().for_each(|entities| {
                entities.iter().for_each(|entity| {
                    entity.apply_rate();
                })
            });

            // apply spawns - despawn
            let mut persistent_spawn: Vec<(&'static str, Vec<PersistentSpawn>)> = Vec::new();
            let mut volatile_spawn: Vec<(&'static str, Vec<VolatileSpawn>)> = Vec::new();
            self.persistent_state
                .persistent_layers
                .values_mut()
                .for_each(|layer| {
                    let len = layer.len();
                    for i in (0..len).rev() {
                        let e = &layer[i];
                        let mut r = e.apply_spawns();
                        if let LivelinessStatus::Despawn = r.alive {
                            debug_assert!(
                                Rc::strong_count(&e.0) == 1,
                                "only the game state is allowed strong references to entities. \
                            inter-entity references should be weak. this possibly leaks"
                            );
                            layer.remove(i);
                        }
                        persistent_spawn.append(&mut r.persistent_spawns);
                        volatile_spawn.append(&mut r.volatile_spawns);
                    }
                });
            self.volatile_layers.values_mut().for_each(|layer| {
                let len = layer.len();
                for i in (0..len).rev() {
                    let e = &layer[i];
                    let mut r = e.apply_spawns();
                    if let LivelinessStatus::Despawn = r.alive {
                        debug_assert!(
                            Rc::strong_count(&e.0) == 1,
                            "only the game state is allowed strong references to entities. \
                        inter-entity references should be weak. this possibly leaks"
                        );
                        layer.remove(i);
                    }
                    volatile_spawn.append(&mut r.volatile_spawns);
                }
            });

            // new spawns
            for s in persistent_spawn {
                let layer = self
                    .persistent_state
                    .persistent_layers
                    .get_mut(&s.0)
                    .expect(&format!(
                        "Entity created persistent spawn for unregistered layer: {}",
                        &s.0
                    ));
                let mut spawned_as_entities =
                    s.1.into_iter()
                        .map(Some)
                        .map(Cell::new)
                        .map(Rc::new)
                        .map(|rc| PersistentEntity(rc))
                        .collect();
                layer.append(&mut spawned_as_entities);
            }
            for s in volatile_spawn {
                let layer = self.volatile_layers.get_mut(&s.0).expect(&format!(
                    "Entity created volatile spawn for unregistered layer: {}",
                    &s.0
                ));
                let mut spawned_as_entities =
                    s.1.into_iter()
                        .map(Some)
                        .map(Cell::new)
                        .map(Rc::new)
                        .map(|rc| VolatileEntity(rc))
                        .collect();
                layer.append(&mut spawned_as_entities);
            }

            self.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
            self.canvas.clear();

            // render all
            self.layer_names.iter().for_each(|layer_name| {
                self.volatile_layers
                    .get(*layer_name)
                    .unwrap()
                    .iter()
                    .for_each(|entity| {
                        entity.render(&mut self.canvas);
                    });
                self.persistent_state
                    .persistent_layers
                    .get(*layer_name)
                    .unwrap()
                    .iter()
                    .for_each(|entity| {
                        entity.render(&mut self.canvas);
                    });
            });

            post_render_hook(&mut self.canvas);

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
