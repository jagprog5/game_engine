use std::{rc::{Rc, Weak}, collections::BTreeMap, cell::Cell, path::Path};

use sdl2::ttf::Font;

/// a shared pointer to font, but also removes the element from the manager
/// if it's no longer being used by anyone else
pub struct FontWrapper<'sdl> {
    // BTree doesn't take hints. would be better than key. cpp interface for
    // this stuff is better :(
    key: (String, u16),
    source: &'sdl FontManager<'sdl>,
    pub font: Rc<Font<'sdl, 'static>>,
}

impl<'sdl> Drop for FontWrapper<'sdl> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.font) == 1 {
            self.source.remove(&self.key);
        }
    }
}

/// typically fonts are the same between ui components
/// this prevents unncessary reload (components share same font object)
pub struct FontManager<'sdl> {
    cache: Cell<BTreeMap<(String, u16), Weak<Font<'sdl, 'static>>>>,
    ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext,
}

impl<'sdl> FontManager<'sdl> {
    pub fn new(ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext) -> Self {
        Self {
            cache: Cell::new(BTreeMap::new()),
            ttf_context,
        }
    }

    // provided for FontWrapper
    pub(crate) fn remove(&self, key: &(String, u16)) {
        let mut cache = self.cache.take();
        cache.remove(key);
    }

    /// get and maybe load a font if it isn't already being used
    pub fn get(&'sdl self, font_path: String, font_size: u16) -> FontWrapper<'sdl> {
        let mut cache = self.cache.take();

        if let Some(weak) = cache.get(&(font_path.clone(), font_size)) {
            // guaranteed from FontWrapper logic
            let rc = weak.upgrade().unwrap();
            let ret = rc.clone();
            self.cache.set(cache);
            return FontWrapper{
                key: (font_path.clone(), font_size),
                source: self,
                font: ret,
            };
        }

        // doesn't exist
        let font = Rc::new(
            self.ttf_context
                .load_font(Path::new(&font_path), font_size)
                .unwrap(),
        );

        cache.insert((font_path.clone(), font_size), Rc::downgrade(&font));
        self.cache.set(cache);
        FontWrapper{
            key: (font_path, font_size),
            source: self,
            font
        }
    }
}
