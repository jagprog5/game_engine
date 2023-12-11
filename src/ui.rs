extern crate sdl2;

pub struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    pub fn includes(&self, point: (i32, i32)) -> bool {
        return point.0 >= self.x
            && point.0 <= self.x + self.width as i32
            && point.1 >= self.y
            && point.1 <= self.y + self.height as i32;
    }
}

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

pub struct UI {
    // layers are rendered front to back. events are only given to the backmost
    // layer, and within that layer the event is processed by each component
    // front to back
    layers: Vec<Vec<Box<dyn UIComponent>>>,
    // this is always kept in sync with the size of the canvas. it is used for the
    // initial call to resize on ui components
    window_size: (u32, u32),
}

impl UI {
    pub fn new(window_size: (u32, u32)) -> Self {
        Self {
            layers: Default::default(),
            window_size,
        }
    }

    /// start the ui with a set of components
    pub fn begin(&mut self, mut layer: Vec<Box<dyn UIComponent>>) {
        if layer.is_empty() {
            return;
        }
        // initialize resize for each component on addition
        layer
            .iter_mut()
            .for_each(|component| component.resize(self.window_size));
        self.layers.push(layer);
    }

    pub fn process(&mut self, e: &sdl2::event::Event) {
        // handle change in window size, propagate to resize each component
        if let sdl2::event::Event::Window {
            timestamp: _,
            window_id: _,
            win_event,
        } = e
        {
            if let sdl2::event::WindowEvent::SizeChanged(x_size, y_size) = win_event {
                self.window_size = (*x_size as u32, *y_size as u32);
                // propagate resize to all components
                self.layers.iter_mut().for_each(|layer| {
                    layer
                        .iter_mut()
                        .for_each(|component| component.resize(self.window_size))
                })
            }
        }

        if self.layers.is_empty() {
            // can't get last layer if empty
            return;
        }

        // result of consumed event
        let mut result: EventHandleResult = EventHandleResult::None;

        let layer_index = self.layers.len() - 1;
        for component in self.layers[layer_index].iter_mut() {
            let r = component.process(e);
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
                    // initialize resize for each component on addition
                    new_layer
                        .iter_mut()
                        .for_each(|component| component.resize(self.window_size));
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
    /// typically called by UI instance
    fn process(&mut self, e: &sdl2::event::Event) -> EventHandleResult;

    /// typically called by UI instance
    fn render(&self, canvas: &mut sdl2::render::WindowCanvas);

    /// this should only be called by UI. recalculate bounds for this component
    /// when it is added to the UI and on window size change.
    fn resize(&mut self, window_size: (u32, u32));
}

/// buttons only recognize left click
pub trait Button: UIComponent {
    fn bounds(&self) -> Rect;

    /// called repeatedly when the mouse is over the button but not being clicked
    fn hover(&mut self);

    /// called repeatedly when the mouse is over the button and being held down
    fn pressed(&mut self);

    /// called once when the mouse is over the button and it is released
    fn released(&mut self) -> EventHandleResult;

    fn process(&mut self, event: &sdl2::event::Event) -> EventHandleResult {
        match event {
            sdl2::event::Event::MouseMotion { x, y, .. } => {
                let bounds = self.bounds();
                if bounds.includes((*x, *y)) {
                    self.hover();
                }
            }
            sdl2::event::Event::MouseButtonDown {
                mouse_btn, x, y, ..
            } => {
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    let bounds = self.bounds();
                    if bounds.includes((*x, *y)) {
                        self.pressed();
                    }
                }
            }
            sdl2::event::Event::MouseButtonUp {
                mouse_btn, x, y, ..
            } => {
                if *mouse_btn == sdl2::mouse::MouseButton::Left {
                    let bounds = self.bounds();
                    if bounds.includes((*x, *y)) {
                        return self.released();
                    }
                }
            }
            _ => {}
        }

        EventHandleResult::None
    }
}
