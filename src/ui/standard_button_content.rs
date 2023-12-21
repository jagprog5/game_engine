use sdl2::{
    image::LoadTexture,
    pixels::Color,
    rect::Rect,
    render::{Texture, TextureCreator, WindowCanvas},
    video::WindowContext,
};

use super::{ui::EventHandleResult, font_manager::FontWrapper};
use super::{standard_button::FocusState, util::shrink_fit, font_manager::FontManager};

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
        font_manager: &'sdl FontManager<'sdl>,
    ) -> (u32, u32);

    fn render(&self, canvas: &mut WindowCanvas, bound: Rect);

    /// where does the entire button go on the screen
    fn get_button_bound(&self, window_size: (u32, u32)) -> Rect;

    fn moved_in(&mut self) {}

    fn moved_out(&mut self) {}

    fn pressed(&mut self) {}

    fn released(&mut self) -> EventHandleResult<'sdl>;
}

pub struct TextContent<'sdl> {
    text: String,
    font_path: String,

    // last height used to generate the font point
    height: u16,

    // font wrapper object needs to be held since on drop they get freed from
    // memory (which would unintentionally reload the font on every resize)
    font: Option<FontWrapper<'sdl>>,
    // dimensions of texture from previous resize
    rendered_dims: (u32, u32),
    rendered_text: Option<Texture<'sdl>>,

    focus_font: Option<FontWrapper<'sdl>>,
    // simple expanding the text doesn't look right, so this always renders a
    // slightly larger font for when the button is focused
    focus_rendered_dims: (u32, u32),
    focus_rendered_text: Option<Texture<'sdl>>,

    functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,

    // used to apply zoom on content
    focus_state: FocusState,
}

impl<'sdl> TextContent<'sdl> {
    const FOCUS_FONT_MULTIPLIER: f32 = 1.025;

    pub fn new(
        text: String,
        font_path: String,
        functional: Box<dyn ContentFunctional<'sdl> + 'sdl>,
    ) -> Self {
        Self {
            text,
            font_path,
            height: 0,
            font: None,
            rendered_dims: (0, 0),
            rendered_text: None,
            focus_font: None,
            focus_rendered_dims: (0, 0),
            focus_rendered_text: None,
            functional,
            focus_state: FocusState::Idle,
        }
    }
}

impl<'sdl> Content<'sdl> for TextContent<'sdl> {
    fn resize(
        &mut self,
        requested_size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_manager: &'sdl FontManager<'sdl>,
    ) -> (u32, u32) {
        self.height = requested_size.1.try_into().unwrap_or(u16::MAX);

        self.font = Some(font_manager.get(self.font_path.clone(), self.height));

        let surface = self.font.as_ref().unwrap().font
            .render(&self.text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();

        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .unwrap();
        let q = texture.query();
        self.rendered_dims = (q.width, q.height);
        self.rendered_text = Some(texture);

        let ret = (q.width, u32::from(self.height));

        // same as above but for the focused text
        let focus_height: f32 = f32::from(self.height) * Self::FOCUS_FONT_MULTIPLIER;
        let focus_height = if focus_height > f32::from(u16::MAX) { u16::MAX } else { focus_height as u16 };

        self.focus_font = Some(font_manager.get(
            self.font_path.clone(),
            focus_height,
        ));

        let surface = self.focus_font.as_ref().unwrap().font
            .render(&self.text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();

        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .unwrap();
        let q = texture.query();
        self.focus_rendered_dims = (q.width, q.height);
        self.focus_rendered_text = Some(texture);

        ret
    }

    fn render(&self, canvas: &mut WindowCanvas, bound: Rect) {
        let dims_to_use = match self.focus_state {
            FocusState::Idle => self.rendered_dims,
            _ => self.focus_rendered_dims,
        };

        let texture_to_use = match self.focus_state {
            FocusState::Idle => &self.rendered_text,
            _ => &self.focus_rendered_text,
        };

        // a font point is defined as the height of the lettering.
        // however, if seems to render some white space above and below as well.
        // this source rectangle gets rid of that
        let y = (dims_to_use.1 as f32 - f32::from(self.height)) / 2f32;
        let src_bound = Rect::new(0i32, y as i32, dims_to_use.0, u32::from(self.height));

        // increase dst bound to match focus scaling
        let dst_bound = match self.focus_state {
            FocusState::Idle => bound,
            _ => {
                let width_expand = bound.width() as f32 * (Self::FOCUS_FONT_MULTIPLIER - 1f32);
                let height_expand = bound.height() as f32 * (Self::FOCUS_FONT_MULTIPLIER - 1f32);
                Rect::new((bound.x as f32 - width_expand / 2f32) as i32, (bound.y as f32 - height_expand / 2f32) as i32,
                    bound.width() + width_expand as u32, bound.height() + height_expand as u32)
            },
        };

        canvas
            .copy(texture_to_use.as_ref().unwrap(), src_bound, dst_bound)
            .unwrap();
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

/// how does the image's dimensions get matched to those of the content
pub enum FitType {
    // naive and simple stretch over destination
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
        _font_manager: &FontManager,
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
            .copy(
                self.rendered_image.as_ref().unwrap(),
                src_bound,
                bound_to_use,
            )
            .unwrap();
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
