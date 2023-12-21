use sdl2::{
    render::{TextureCreator, WindowCanvas},
    video::WindowContext,
};

use super::font_manager::FontManager;

extern crate sdl2;

pub enum EventHandleResult<'sdl> {
    /// if an event is "consumed", this means that it will not be processed by other ui components.\
    /// None indicates that the event is not consumed; it will pass to other ui components in the backmost layer.
    /// all other variant consume the events
    None,

    /// exit only the current layer of the ui
    RemoveLayer,

    /// add layer. if it is empty then this does nothing
    AddLayer(Vec<Box<dyn UIComponent<'sdl> + 'sdl>>),

    /// if this is empty then it acts like RemoveLayer
    ReplaceLayer(Vec<Box<dyn UIComponent<'sdl> + 'sdl>>),

    /// all levels of the UI should be exited
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
    font_manager: &'sdl FontManager<'sdl>,

    /// state shared by all ui components
    state: UIState,
}

impl<'sdl> UI<'sdl> {
    pub fn new(
        canvas: &WindowCanvas,
        font_manager: &'sdl FontManager<'sdl>,
        texture_creator: &'sdl TextureCreator<WindowContext>,
    ) -> Result<Self, String> {
        Ok(Self {
            layers: Default::default(),
            texture_creator,
            state: UIState {
                window_size: canvas.output_size().unwrap(),
                button_down: false,
            },
            font_manager,
        })
    }

    fn replace_top(&mut self, mut layer: Vec<Box<dyn UIComponent<'sdl> + 'sdl>>) {
        // resize for each component on addition to the ui
        layer.iter_mut().for_each(|component| {
            component.resize(
                self.state.window_size,
                self.texture_creator,
                self.font_manager,
            )
        });

        *self.layers.last_mut().unwrap() = layer;
    }

    /// push a layer to the ui
    pub fn add(&mut self, layer: Vec<Box<dyn UIComponent<'sdl> + 'sdl>>) {
        self.private_add(layer, None)
    }

    fn private_add(&mut self, mut layer: Vec<Box<dyn UIComponent<'sdl> + 'sdl>>, mouse_position: Option<(i32, i32)>) {
        if layer.is_empty() {
            return;
        }

        // let the top layer know it has been exited
        self.layers.last_mut().map(|prior_layer| {
            prior_layer
                .iter_mut()
                .for_each(|component| component.covered())
        });

        // resize for each component on addition to the ui
        layer.iter_mut().for_each(|component| {
            component.resize(
                self.state.window_size,
                self.texture_creator,
                self.font_manager,
            )
        });

        layer.iter_mut().for_each(|component| {
            component.entered(mouse_position);
        });

        // add it
        self.layers.push(layer);
    }

    /// returns false if run is complete
    pub fn process(&mut self, e: &sdl2::event::Event) -> bool {
        // the back most layer must never be empty
        debug_assert!(self.layers.last().map_or(true, |layer| !layer.is_empty()));

        // there is some logic which is handled by the UI as a whole, and not any
        // individual components. this is stored in self.state

        // record mouse pos for if a layer is removed.
        let mut entered_pos: Option<(i32, i32)> = None;

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
                                &self.font_manager,
                            )
                        })
                    })
                }
            }
            sdl2::event::Event::MouseButtonDown {
                mouse_btn, x, y, ..
            } => {
                entered_pos = Some((*x, *y));
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    self.state.button_down = true;
                }
            }
            sdl2::event::Event::MouseButtonUp {
                mouse_btn, x, y, ..
            } => {
                entered_pos = Some((*x, *y));
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    self.state.button_down = false;
                }
            }
            sdl2::event::Event::MouseMotion { x, y, .. } => {
                entered_pos = Some((*x, *y));
            }
            sdl2::event::Event::MouseWheel { x, y, .. } => {
                entered_pos = Some((*x, *y));
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

            EventHandleResult::Clear => {
                self.layers.clear();
            }
            EventHandleResult::Quit => return false,
            EventHandleResult::RemoveLayer => {
                self.layers.pop();
                self.layers.last_mut().map(|layer| {
                    layer.iter_mut().for_each(|component| {
                        component.entered(entered_pos);
                    })
                });
            }
            EventHandleResult::AddLayer(layer) => {
                self.private_add(layer, entered_pos);
            }
            EventHandleResult::ReplaceLayer(layer) => {
                if layer.is_empty() {
                    // treated the exact same as remove layer
                    self.layers.pop();
                } else {
                    self.replace_top(layer);
                }
                self.layers.last_mut().map(|layer| {
                    layer.iter_mut().for_each(|component| {
                        component.entered(entered_pos);
                    })
                });
            }
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
        font_manager: &'sdl FontManager<'sdl>,
    );

    /// a event that occurs when this component was part of the top most layer and another layer was added
    /// on top of it
    fn covered(&mut self) {}

    /// indicates that this component is now in the top most layer
    /// returns true if it influenced the state of this component
    fn entered(&mut self, _mouse_position: Option<(i32, i32)>) -> bool {
        false
    }
}

/// this is a minimal wrapper around UIComponent which handles mouse logic. it
/// only recognizes left click
pub trait Button<'sdl>: UIComponent<'sdl> {
    fn bounds(&self) -> sdl2::rect::Rect;

    /// called repeatedly if the mouse is not over the button
    fn moved_out(&mut self);

    /// called repeatedly if the mouse is over the button and left click isn't down
    fn moved_in(&mut self);

    /// when a layer is entered and the mouse is over a button, then this is
    /// called instead of a typical moved_in. typically this is treated the same
    /// as moved_in
    fn moved_in_from_entered(&mut self) {
        self.moved_in()
    }

    /// called repeatedly if the mouse is over the button and left click is pressed down
    fn pressed(&mut self);

    /// called once when mouse is on this button and left click is released
    fn released(&mut self) -> EventHandleResult<'sdl>;

    fn covered(&mut self) {
        self.moved_out();
    }

    fn entered(&mut self, mouse_position: Option<(i32, i32)>) -> bool {
        mouse_position.map_or(false, |pos| {
            let bounds = self.bounds();
            if bounds.contains_point((pos.0, pos.1)) {
                self.moved_in_from_entered();
                return true;
            }
            false
        })
    }

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
