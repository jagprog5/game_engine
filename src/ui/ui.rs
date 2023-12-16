use std::{num::NonZeroUsize, path::Path, rc::Rc};

use lru::LruCache;
use sdl2::{
    render::{TextureCreator, WindowCanvas},
    ttf::{Font, Sdl2TtfContext},
    video::WindowContext,
};

extern crate sdl2;

pub enum EventHandleResult<'sdl> {
    /// if an event is "consumed", this means that it will not be processed by other ui components.\
    /// None indicates that the event is not consumed; it will pass to other ui components in the backmost layer
    None,
    /// for removal or replacement of current layer\
    /// Some indicates that the event is consumed and the backmost ui layer is removed, and replaced with the contained value only if not empty.
    Some(Vec<Box<dyn UIComponent<'sdl> + 'sdl>>),
    /// indicates event is consumed and that all levels of the UI should be exited
    Clear,
    /// indicates that the game should exit
    Quit,
}

/// when events are processed, shared info between all UI components within a UI
pub struct UIState {
    /// always kept in sync with the left mouse button
    pub button_down: bool,
    /// always kept in sync with the size of the canvas. it is used for the
    /// calls to resize on ui components
    pub window_size: (u32, u32),
}

/// sdl is the lifetime of various borrowed structs. this includes the texture_creator and the ttf context.
/// they will be needed through the lifetime of this ui instance
pub struct UI<'sdl> {
    // layers are rendered front to back. events are only given to the backmost
    // layer, and within that layer the event is processed by each component
    // front to back
    layers: Vec<Vec<Box<dyn UIComponent<'sdl> + 'sdl>>>,

    texture_creator: &'sdl TextureCreator<WindowContext>,
    font_manager: FontCache<'sdl>,

    /// always kept in sync with the left mouse button
    state: UIState,
}

impl<'sdl> UI<'sdl> {
    pub fn new(
        canvas: &WindowCanvas,
        ttf_context: &'sdl Sdl2TtfContext,
        texture_creator: &'sdl TextureCreator<WindowContext>,
    ) -> Result<Self, String> {
        Ok(Self {
            layers: Default::default(),
            texture_creator,
            state: UIState {
                window_size: canvas.output_size().unwrap(),
                button_down: false,
            },
            font_manager: FontCache::new(16, ttf_context),
        })
    }

    /// push a layer to the ui
    pub fn add(&mut self, mut layer: Vec<Box<dyn UIComponent<'sdl> + 'sdl>>) {
        if layer.is_empty() {
            return;
        }
        // initialize resize for each component on addition
        layer.iter_mut().for_each(|component| {
            component.resize(
                self.state.window_size,
                self.texture_creator,
                &mut self.font_manager,
            )
        });
        self.layers.push(layer);
    }

    /// returns false if run is complete
    pub fn process(&mut self, e: &sdl2::event::Event) -> bool {
        // there is some logic which is handled by the UI as a whole, and not any
        // individual components. this is stored in self.state
        match e {
            sdl2::event::Event::Window {
                timestamp: _,
                window_id: _,
                win_event,
            } => {
                // on change of window size keep self.window_size in sync and propagate
                // it to components
                if let sdl2::event::WindowEvent::SizeChanged(x_size, y_size) = win_event {
                    self.state.window_size = (*x_size as u32, *y_size as u32);
                    // propagate resize to all components
                    self.layers.iter_mut().for_each(|layer| {
                        layer.iter_mut().for_each(|component| {
                            component.resize(
                                self.state.window_size,
                                &self.texture_creator,
                                &mut self.font_manager,
                            )
                        })
                    })
                }
            }
            sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => {
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    self.state.button_down = true;
                }
            }
            sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => {
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    self.state.button_down = false;
                }
            }
            _ => {}
        } // end of share ui state update

        // propagate events to last layer
        let layer = match self.layers.last_mut() {
            Some(layer) => layer,
            None => return true, // can't get last layer if empty
        };

        // result of consumed event
        let mut result = EventHandleResult::None;

        for component in layer.iter_mut() {
            let r: EventHandleResult = component.process(&self.state, e);
            if let EventHandleResult::None = r {
                continue;
            }
            result = r;
            break;
        }

        match result {
            EventHandleResult::None => {}
            EventHandleResult::Some(new_layer) => {
                self.layers.pop();
                self.add(new_layer);
            }
            EventHandleResult::Clear => {
                self.layers.clear();
            }
            EventHandleResult::Quit => return false,
        }

        true
    }

    pub fn render(&self, canvas: &mut WindowCanvas) {
        self.layers
            .iter()
            .for_each(|layer| layer.iter().for_each(|component| component.render(canvas)));
    }
}

pub trait UIComponent<'sdl> {
    /// called by UI instance
    fn process(&mut self, ui_state: &UIState, e: &sdl2::event::Event) -> EventHandleResult<'sdl>;

    /// called by UI instance
    fn render(&self, canvas: &mut WindowCanvas);

    /// this should only be called by UI. recalculate bounds for this component and render any graphics.\
    /// this is called when it is initially added to the ui and
    /// each time the window changes size.
    fn resize(
        &mut self,
        window_size: (u32, u32),
        texture_creator: &'sdl TextureCreator<WindowContext>,
        font_cache: &mut FontCache,
    );
}

/// buttons only recognize left click
pub trait Button<'sdl>: UIComponent<'sdl> {
    fn bounds(&self) -> sdl2::rect::Rect;

    /// called repeatedly if the mouse is not over the button
    fn moved_out(&mut self);

    /// called repeatedly if the mouse is over the button and left click isn'texture_creator down
    fn moved_in(&mut self);

    /// called repeatedly if the mouse is over the button and left click is pressed down
    fn pressed(&mut self);

    /// called once when mouse if on button and left click is released
    fn released(&mut self) -> EventHandleResult<'sdl>;

    fn process(
        &mut self,
        ui_state: &UIState,
        event: &sdl2::event::Event,
    ) -> EventHandleResult<'sdl> {
        match event {
            sdl2::event::Event::MouseMotion { x, y, .. } => {
                let bounds = self.bounds();
                if !bounds.contains_point((*x, *y)) {
                    self.moved_out();
                    return EventHandleResult::None;
                }

                if ui_state.button_down {
                    self.pressed()
                } else {
                    self.moved_in()
                }
                return EventHandleResult::None;
            }
            sdl2::event::Event::MouseButtonDown {
                mouse_btn, x, y, ..
            } => {
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    let bounds = self.bounds();
                    if bounds.contains_point((*x, *y)) {
                        self.pressed();
                    }
                }
            }
            sdl2::event::Event::MouseButtonUp {
                mouse_btn, x, y, ..
            } => {
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    let bounds = self.bounds();
                    if bounds.contains_point((*x, *y)) {
                        let r = self.released();
                        self.moved_in();
                        return r;
                    }
                }
            }
            _ => {}
        }

        EventHandleResult::None
    }
}

// typically fonts are the same between ui components
// this prevents unncessary reload (components share same font object)
pub struct FontCache<'sdl> {
    cache: LruCache<(String, u16), Rc<Font<'sdl, 'static>>>,
    ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext,
}

impl<'sdl> FontCache<'sdl> {
    pub fn new(capacity: usize, ttf_context: &'sdl sdl2::ttf::Sdl2TtfContext) -> Self {
        assert!(capacity != 0);
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            ttf_context,
        }
    }

    /// get and maybe load a font if it's not in the cache./
    /// weak ref can safely be unwrapped if there has not yet been another call to get
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
