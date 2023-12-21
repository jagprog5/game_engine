use std::{rc::Rc, num::NonZeroUsize, path::Path};

use lru::LruCache;
use sdl2::ttf::Font;

// typically fonts are the same between ui components
// this prevents unncessary reload (components share same font object)
pub struct FontCache<'sdl> {
    cache: LruCache<(String, u16), Rc<Font<'sdl, 'static>>>,
    ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext,
}

impl<'sdl> FontCache<'sdl> {
    pub fn new(capacity: usize, ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            ttf_context,
        }
    }

    /// get and maybe load a font if it's not in the cache
    pub fn get(&mut self, font_path: String, font_size: u16) -> Rc<Font<'sdl, 'static>> {
        if let Some(rc) = self.cache.get(&(font_path.clone(), font_size)) {
            return rc.clone();
        }

        // does not already exist
        let font = Rc::new(
            self.ttf_context
                .load_font(Path::new(&font_path), font_size)
                .unwrap(),
        );
        self.cache.put((font_path, font_size), font.clone());
        font
    }
}
