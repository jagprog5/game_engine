use std::{cell::Cell, marker::PhantomData, num::NonZeroUsize, rc::Rc};

use lru::LruCache;
use sdl2::mixer::Chunk;

/// cache of chunks (fully decoded into memory sound files)
/// sdl is the lifetime of the sdl audio constructs (imposed artificially here)
pub struct ChunkCache<'sdl> {
    cache: Cell<Option<Box<LruCache<String, Rc<Chunk>>>>>,
    _mark: PhantomData<&'sdl ()>,
}

impl<'sdl> ChunkCache<'sdl> {
    pub fn new(
        capacity: usize,
        _audio: &'sdl sdl2::AudioSubsystem,
        _mixer: &'sdl sdl2::mixer::Sdl2MixerContext,
    ) -> Self {
        Self {
            cache: Cell::new(Some(Box::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap(),
            )))),
            _mark: PhantomData::<&'sdl ()>,
        }
    }

    /// get and maybe load chunk
    pub fn get(&mut self, sound_path: String) -> Rc<Chunk> {
        let mut cache = self.cache.take().unwrap();

        if let Some(rc) = cache.get(&sound_path) {
            let ret = rc.clone();
            self.cache.set(Some(cache));
            return ret;
        }

        // does not already exist
        let sound = Rc::new(sdl2::mixer::Chunk::from_file(sound_path.clone()).unwrap());
        cache.put(sound_path.clone(), sound.clone());
        self.cache.set(Some(cache));
        sound
    }
}
