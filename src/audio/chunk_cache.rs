use std::{num::NonZeroUsize, rc::Rc};

use lru::LruCache;
use sdl2::mixer::Chunk;

/// cache of chunks (fully decoded into memory sound files)
pub struct ChunkCache {
    cache: LruCache<String, Rc<Chunk>>,
}

impl ChunkCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    /// get and maybe load chunk
    pub fn get(&mut self, sound_path: String) -> Rc<Chunk> {
        if let Some(rc) = self.cache.get(&sound_path) {
            return rc.clone();
        }

        // does not already exist
        let sound = Rc::new(sdl2::mixer::Chunk::from_file(sound_path.clone()).unwrap());
        self.cache.put(sound_path.clone(), sound.clone());
        sound
    }
}
