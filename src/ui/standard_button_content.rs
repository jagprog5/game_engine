use sdl2::{
    image::LoadTexture,
    pixels::Color,
    rect::Rect,
    render::{Texture, TextureCreator, WindowCanvas},
    video::WindowContext,
};

use super::{EventHandleResult, FontCache};

pub trait ContentFunctional<'sdl> {
    /// what happens when the button is released
    fn released(&mut self) -> EventHandleResult<'sdl>;

    /// where does the entire button go on the screen
    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect;
}

/// information specific to each standard button instance:
///  - where is the button, and how big is it\
///  - what happens when the button is released\
///  - what is drawn within the border of the standard button
pub trait Content<'sdl> {
    /// returns the size of the inner content (not including the border)
    fn resize_inner(
        &mut self,
        requested_size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_cache: &mut FontCache,
    ) -> (u32, u32);

    fn render_inner(&self, canvas: &mut WindowCanvas, bound: Rect);

    /// what happens when the button is released
    fn released(&mut self) -> EventHandleResult<'sdl>;

    /// where does the entire button go on the screen
    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect;
}

pub struct TextContent<'sdl> {
    text: String,
    font_path: String,

    height: u16, // last height used to generate the font for this instance
    rendered_text_height: u32, // the height of the texture
    rendered_text: Option<Texture<'sdl>>,
    functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,
}

impl<'sdl> TextContent<'sdl> {
    pub fn new(
        text: String,
        font_path: String,
        functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,
    ) -> Self {
        Self {
            text,
            font_path,
            height: 0,
            rendered_text_height: 0,
            rendered_text: None,
            functional,
        }
    }
}

impl<'sdl> Content<'sdl> for TextContent<'sdl> {
    fn resize_inner(
        &mut self,
        requested_size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_cache: &mut FontCache,
    ) -> (u32, u32) {
        self.height = requested_size.1.try_into().unwrap_or(u16::MAX);
        let font_rc = font_cache.get(self.font_path.clone(), self.height);
        let surface = font_rc
            .render(&self.text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();

        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .unwrap();
        let q = texture.query();
        self.rendered_text_height = q.height;
        self.rendered_text = Some(texture);
        (q.width, u32::from(self.height))
    }

    fn render_inner(&self, canvas: &mut WindowCanvas, bound: Rect) {
        // a font point is defined as the height of the lettering.
        // however, if seems to render some white space above and below as well.
        // this source rectangle gets rid of that
        let height = u32::from(self.height);
        let y = (self.rendered_text_height / 2 - height / 2) as i32;
        canvas
            .copy(self.rendered_text.as_ref().unwrap(), Rect::new(0, y, u32::MAX, height), bound)
            .unwrap();
    }

    fn released(&mut self) -> EventHandleResult<'sdl> {
        self.functional.released()
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        self.functional.get_button_bound(window_size)
    }
}

pub struct ImageContent<'sdl> {
    img_path: String,
    rendered_image: Option<Texture<'sdl>>,
    functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,
}

impl<'sdl> ImageContent<'sdl> {
    pub fn new(img_path: String, functional: Box<dyn ContentFunctional<'sdl> + 'sdl>) -> Self {
        Self {
            img_path,
            rendered_image: None,
            functional,
        }
    }
}

impl<'sdl> Content<'sdl> for ImageContent<'sdl> {
    fn resize_inner(
        &mut self,
        size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        _font_cache: &mut FontCache,
    ) -> (u32, u32) {
        // todo stretching / interpolation / cut off. something to make it look better with aspect ratio.
        if let None = self.rendered_image {
            self.rendered_image =
                Some(texture_creator.load_texture(self.img_path.clone()).unwrap());
        }
        size
    }

    fn render_inner(&self, canvas: &mut WindowCanvas, bound: Rect) {
        canvas
            .copy(self.rendered_image.as_ref().unwrap(), None, bound)
            .unwrap();
    }

    fn released(&mut self) -> EventHandleResult<'sdl> {
        self.functional.released()
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        self.functional.get_button_bound(window_size)
    }
}