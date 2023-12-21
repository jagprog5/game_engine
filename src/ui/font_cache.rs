use std::{cell::Cell, num::NonZeroUsize, path::Path, rc::Rc};

use lru::LruCache;
use sdl2::ttf::Font;

// todo replace lru with map to weak rc

// typically fonts are the same between ui components
// this prevents unncessary reload (components share same font object)
pub struct FontCache<'sdl> {
    cache: Cell<
        Option<
            Box< // todo doesn't have to be rc here anymore
                LruCache<(String, u16), Rc<Font<'sdl, 'static>>
                >
            >
        >
    >,
    ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext,
}

impl<'sdl> FontCache<'sdl> {
    pub fn new(capacity: usize, ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext) -> Self {
        Self {
            cache: Cell::new(Some(Box::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap(),
            )))),
            ttf_context,
        }
    }

    /// get and maybe load a font if it's not in the cache
    pub fn get(&self, font_path: String, font_size: u16) -> Rc<Font<'sdl, 'static>> {
        let mut cache = self.cache.take().unwrap();

        if let Some(rc) = cache.get(&(font_path.clone(), font_size)) {
            let ret = rc.clone();
            self.cache.set(Some(cache));
            return ret;
        }

        // does not already exist
        let font = Rc::new(
            self.ttf_context
                .load_font(Path::new(&font_path), font_size)
                .unwrap(),
        );
        cache.put((font_path, font_size), font.clone());
        self.cache.set(Some(cache));
        font
    }
}
