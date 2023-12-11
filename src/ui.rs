extern crate sdl2;

pub enum EventHandleResult {
    /// if an event is "consumed", this means that it will not be processed by other ui components.\
    /// None indicates that the event is not consumed; it will pass to other ui components in the backmost layer
    None,
    /// for removal or replacement of current layer\
    /// Some indicates that the event is consumed and the backmost ui layer is removed, and replaced with the contained value only if not empty.
    Some(Vec<Box<dyn UIComponent>>),
    /// indicates event is consumed and that all levels of the UI should be exited
    Clear,
}

/// this includes any state which is shared between all components of the UI
pub struct UIState {
    /// always kept in sync with the left mouse button
    pub button_down: bool,
    /// always kept in sync with the size of the canvas. it is used for the
    /// initial call to resize on ui components
    pub window_size: (u32, u32),
}

pub struct UI {
    // layers are rendered front to back. events are only given to the backmost
    // layer, and within that layer the event is processed by each component
    // front to back
    layers: Vec<Vec<Box<dyn UIComponent>>>,
    /// always kept in sync with the left mouse button
    pub state: UIState,
}

impl UI {
    /// the initial window size
    pub fn new(window_size: (u32, u32)) -> Self {
        Self {
            layers: Default::default(),
            state: UIState {
                window_size,
                button_down: false,
            },
        }
    }

    /// push a layer to the ui
    pub fn add(&mut self, mut layer: Vec<Box<dyn UIComponent>>) {
        if layer.is_empty() {
            return;
        }
        // initialize resize for each component on addition
        layer
            .iter_mut()
            .for_each(|component| component.resize(self.state.window_size));
        self.layers.push(layer);
    }

    pub fn process(&mut self, e: &sdl2::event::Event) {
        // there is some logic which is handled by the UI as a whole, and not any
        // individual components. this is stored in self.state
        match e {
            sdl2::event::Event::Window {
                timestamp: _,
                window_id: _,
                win_event,
            } => {
                // on change of window size keep self.window_size in sync and propogate
                // it to components
                if let sdl2::event::WindowEvent::SizeChanged(x_size, y_size) = win_event {
                    self.state.window_size = (*x_size as u32, *y_size as u32);
                    // propagate resize to all components
                    self.layers.iter_mut().for_each(|layer| {
                        layer
                            .iter_mut()
                            .for_each(|component| component.resize(self.state.window_size))
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

        // propagate events to layers
        if self.layers.is_empty() {
            // can't get last layer if empty
            return;
        }

        // result of consumed event
        let mut result: EventHandleResult = EventHandleResult::None;

        let layer_index = self.layers.len() - 1;
        let state = &self.state;
        for component in self.layers[layer_index].iter_mut() {
            let r = component.process(state, e);
            if let EventHandleResult::None = r {
                continue;
            }
            result = r;
            break;
        }

        match result {
            EventHandleResult::None => {}
            EventHandleResult::Some(mut new_layer) => {
                self.layers.pop();
                if !new_layer.is_empty() {
                    // initialize resize for each added component
                    new_layer
                        .iter_mut()
                        .for_each(|component| component.resize(self.state.window_size));
                    self.layers.push(new_layer);
                }
            }
            EventHandleResult::Clear => {
                self.layers.clear();
            }
        }
    }

    pub fn render(&self, canvas: &mut sdl2::render::WindowCanvas) {
        self.layers
            .iter()
            .for_each(|layer| layer.iter().for_each(|component| component.render(canvas)));
    }
}

pub trait UIComponent {
    /// called by UI instance
    fn process(&mut self, ui_state: &UIState, e: &sdl2::event::Event) -> EventHandleResult;

    /// called by UI instance
    fn render(&self, canvas: &mut sdl2::render::WindowCanvas);

    /// this should only be called by UI. recalculate bounds for this component
    /// when it is added to the UI and on window size change.
    fn resize(&mut self, window_size: (u32, u32));
}

/// buttons only recognize left click
pub trait Button: UIComponent {
    fn bounds(&self) -> sdl2::rect::Rect;

    /// called repeatedly if the mouse is not over the button
    fn moved_out(&mut self);

    /// called repeatedly if the mouse is over the button and left click isn't down
    fn moved_in(&mut self);

    /// called repeatedly if the mouse is over the button and left click is pressed down
    fn pressed(&mut self);

    /// called once when mouse if on button and left click is released
    fn released(&mut self) -> EventHandleResult;

    fn process(&mut self, ui_state: &UIState, event: &sdl2::event::Event) -> EventHandleResult {
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
                        self.released();
                        self.moved_in()
                    }
                }
            }
            _ => {}
        }

        EventHandleResult::None
    }
}
