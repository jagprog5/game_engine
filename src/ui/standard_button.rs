use sdl2::{pixels::Color, rect::Rect, render::WindowCanvas};

use super::{standard_button_content::Content, Button, EventHandleResult, UIComponent, UIState};

#[derive(PartialEq, Eq)]
// button state for rendering, changed by sdl events
pub enum FocusState {
    Idle,
    Hovered,
    Pressed,
}

/// a standard button has a gradient border and a background color. the stuff
/// inside the border is a Content. the content also handles the position and
/// size for this button, as well as what happen on button release
pub struct StandardButton<'sdl> {
    bg_idle_color: Color,
    bg_pressed_color: Color,
    border_outer_color: Color,
    border_inner_color: Color,
    border_width: u16,
    border_steps: u16,
    content: Box<dyn Content<'sdl> + 'sdl>,
    bound: Rect,
    content_bound: Rect,
    focus_state: FocusState,
}

impl<'sdl> StandardButton<'sdl> {
    pub fn default_look(content: Box<dyn Content<'sdl> + 'sdl>) -> Self {
        Self::new(
            Color::RGBA(100, 100, 100, 30),
            Color::RGBA(100, 100, 100, 100),
            Color::RGB(150, 150, 150),
            Color::RGB(50, 50, 50),
            15,
            4,
            content,
        )
    }

    pub fn new(
        bg_idle_color: Color,
        bg_pressed_color: Color,
        border_outer_color: Color,
        border_inner_color: Color,
        border_width: u16,
        border_steps: u16,
        content: Box<dyn Content<'sdl> + 'sdl>,
    ) -> Self {
        Self {
            bg_idle_color,
            bg_pressed_color,
            border_outer_color,
            border_inner_color,
            border_width,
            border_steps,
            content,

            // these bounds will never be used (replaced on resize when added to UI),
            // but just in case this places it off screen. Rect::contains_point is
            // inclusive
            content_bound: Rect::new(-1, -1, 0, 0),
            bound: Rect::new(-1, -1, 0, 0),
            focus_state: FocusState::Idle,
        }
    }
}

impl<'sdl> UIComponent<'sdl> for StandardButton<'sdl> {
    fn process(&mut self, ui_state: &UIState, e: &sdl2::event::Event) -> EventHandleResult<'sdl> {
        Button::<'sdl>::process(self, ui_state, e)
    }

    fn render(&self, canvas: &mut WindowCanvas) {
        let bg_color = match self.focus_state {
            FocusState::Idle => self.bg_idle_color,
            FocusState::Hovered => {
                super::util::interpolate_color(self.bg_idle_color, self.bg_pressed_color, 0.5)
            }
            FocusState::Pressed => self.bg_pressed_color,
        };

        canvas.set_draw_color(bg_color);

        self.content.render(canvas, self.content_bound);

        canvas.fill_rect(self.bound).unwrap();
        super::util::render_gradient_border(
            canvas,
            self.bound,
            self.border_outer_color,
            self.border_inner_color,
            self.border_width,
            self.border_steps,
        );
    }

    fn resize(
        &mut self,
        window_size: (u32, u32),
        texture_creator: &'sdl sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        font_cache: &mut super::FontCache,
    ) {
        // how big will the entire button be
        let bound = self.content.get_button_bound(window_size);
        let center = (bound.x + bound.w / 2, bound.y + bound.h / 2);

        // create a size which is enclosed by the border. pass that to the inner content
        let requested_content_bound_w = bound
            .width()
            .checked_sub(u32::from(self.border_width) * 2)
            .unwrap_or(bound.width());
        let requested_content_bound_h = bound
            .height()
            .checked_sub(u32::from(self.border_width) * 2)
            .unwrap_or(bound.height());

        // have the content re-render its texture based on the space available
        // inside the button, not including the border. see how much space that took
        let responded_dim = self.content.resize(
            (requested_content_bound_w, requested_content_bound_h),
            texture_creator,
            font_cache,
        );

        self.content_bound = Rect::new(
            center.0 - (responded_dim.0 / 2) as i32,
            center.1 - (responded_dim.1 / 2) as i32,
            responded_dim.0,
            responded_dim.1,
        );

        // add the border back on top of the inner content responded dimensions.
        // this expands about the center of the button
        let responded_dim = (
            responded_dim.0 + u32::from(self.border_width) * 2,
            responded_dim.1 + u32::from(self.border_width) * 2,
        );

        // expands about center if the inner
        self.bound = Rect::new(
            center.0 - (responded_dim.0 / 2) as i32,
            center.1 - (responded_dim.1 / 2) as i32,
            responded_dim.0,
            responded_dim.1,
        );
    }

    fn exited_layer(&mut self) {
        Button::<'sdl>::exited_layer(self);
        self.content.moved_out();
    }

    fn entered_layer(&mut self, mouse_position: Option<(i32, i32)>) -> bool {
        if Button::<'sdl>::entered_layer(self, mouse_position) {
            self.content.moved_in();
            return true;
        }
        false
    }
}

impl<'sdl> Button<'sdl> for StandardButton<'sdl> {
    fn bounds(&self) -> Rect {
        self.bound
    }

    fn moved_out(&mut self) {
        self.focus_state = FocusState::Idle;
        self.content.moved_out();
    }

    fn moved_in(&mut self) {
        self.focus_state = FocusState::Hovered;
        self.content.moved_in();
    }

    fn pressed(&mut self) {
        self.focus_state = FocusState::Pressed;
        self.content.pressed();
    }

    fn released(&mut self) -> EventHandleResult<'sdl> {
        self.content.released()
    }
}
