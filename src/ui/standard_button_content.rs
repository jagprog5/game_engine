use sdl2::{
    image::LoadTexture,
    pixels::Color,
    rect::Rect,
    render::{Texture, TextureCreator, WindowCanvas},
    video::WindowContext,
};

use super::{standard_button::FocusState, util::shrink_fit, EventHandleResult, FontCache};

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
    fn resize(
        &mut self,
        requested_size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_cache: &mut FontCache,
    ) -> (u32, u32);

    fn render(&self, canvas: &mut WindowCanvas, bound: Rect);

    /// where does the entire button go on the screen
    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect;

    /// following functions are forwarded to by standard button
    fn reset(&mut self) {}

    fn moved_in(&mut self) {}

    fn moved_out(&mut self) {}

    fn pressed(&mut self) {}

    fn released(&mut self) -> EventHandleResult<'sdl>;
}

pub struct TextContent<'sdl> {
    text: String,
    font_path: String,

    height: u16,               // last height used to generate the font for this instance
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
    fn resize(
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

    fn render(&self, canvas: &mut WindowCanvas, bound: Rect) {
        // a font point is defined as the height of the lettering.
        // however, if seems to render some white space above and below as well.
        // this source rectangle gets rid of that
        let height = u32::from(self.height);
        let y = (self.rendered_text_height / 2 - height / 2) as i32;
        canvas
            .copy(
                self.rendered_text.as_ref().unwrap(),
                Rect::new(0, y, u32::MAX, height),
                bound,
            )
            .unwrap();
    }

    fn released(&mut self) -> EventHandleResult<'sdl> {
        self.functional.released()
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        self.functional.get_button_bound(window_size)
    }
}

/// how does the image's dimensions get matched to those of the content
pub enum FitType {
    // native and simple stretch over destination
    Stretch,
    // shrink in output to match aspect ratio
    Shrink,
}

pub struct ImageContent<'sdl> {
    img_path: String,
    image_dims: (u32, u32),
    rendered_image: Option<Texture<'sdl>>,
    functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,
    fit_type: FitType,
    // the amount that the button zooms is when it is focused.
    // some of the zoom happens on hover, and all on click
    border_zoom_portion: f32,
    // used in conjunction with the border zoom
    focus_state: FocusState,
}

impl<'sdl> ImageContent<'sdl> {
    pub fn new(
        img_path: String,
        functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,
        fit_type: FitType,
        border_zoom_portion: f32,
    ) -> Self {
        Self {
            img_path,
            image_dims: (0, 0),
            rendered_image: None,
            functional,
            fit_type,
            border_zoom_portion,
            focus_state: FocusState::Idle,
        }
    }
}

impl<'sdl> Content<'sdl> for ImageContent<'sdl> {
    fn resize(
        &mut self,
        size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        _font_cache: &mut FontCache,
    ) -> (u32, u32) {
        if let None = self.rendered_image {
            self.rendered_image =
                Some(texture_creator.load_texture(self.img_path.clone()).unwrap());
            let q = self.rendered_image.as_ref().unwrap().query();
            self.image_dims = (q.width, q.height);
        }
        size
    }

    fn render(&self, canvas: &mut WindowCanvas, bound: Rect) {
        let bound_to_use = match self.fit_type {
            FitType::Stretch => bound,
            FitType::Shrink => shrink_fit(self.image_dims, bound),
        };
        let border_zoom_to_use = match self.focus_state {
            FocusState::Idle => 0f32,
            _ => self.border_zoom_portion,
        };
        let border_zoom_width = border_zoom_to_use * self.image_dims.0 as f32;
        let border_zoom_height = border_zoom_to_use * self.image_dims.1 as f32;
        let src_bound = Rect::new(
            border_zoom_width as i32,
            border_zoom_height as i32,
            (self.image_dims.0 as f32 - border_zoom_width * 2f32) as u32,
            (self.image_dims.1 as f32 - border_zoom_width * 2f32) as u32,
        );
        canvas
            .copy(self.rendered_image.as_ref().unwrap(), src_bound, bound_to_use)
            .unwrap();
    }

    fn reset(&mut self) {
        self.focus_state = FocusState::Idle;
    }

    fn moved_in(&mut self) {
        self.focus_state = FocusState::Hovered;
    }

    fn moved_out(&mut self) {
        self.focus_state = FocusState::Idle;
    }

    fn pressed(&mut self) {
        self.focus_state = FocusState::Pressed;
    }

    fn released(&mut self) -> EventHandleResult<'sdl> {
        self.functional.released()
    }

    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect {
        self.functional.get_button_bound(window_size)
    }
}
