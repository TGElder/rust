use std::sync::Arc;

use coords::*;
use cursor_handler::*;
use event_handlers::*;
use events::{EventConsumer, EventHandler, EventHandlerAdapter};
use graphics::{Drawing, GraphicsEngine};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use glutin::GlContext;

#[derive(Debug, PartialEq)]
pub enum Button {
    Key(glutin::VirtualKeyCode),
    Mouse(glutin::MouseButton),
}

#[derive(Debug)]
pub enum Event {
    Start,
    Tick,
    Shutdown,
    Resize(glutin::dpi::PhysicalSize),
    DPIChanged(f64),
    CursorMoved(GLCoord4D),
    WorldPositionChanged(WorldCoord),
    GlutinEvent(glutin::Event),
    Drag(GLCoord4D),
    Button {
        button: Button,
        state: glutin::ElementState,
        modifiers: glutin::ModifiersState,
    },
}

#[derive(Debug)]
pub enum Command {
    Shutdown,
    Resize(glutin::dpi::PhysicalSize),
    Translate(GLCoord2D),
    Scale {
        center: GLCoord4D,
        scale: GLCoord2D,
    },
    Rotate {
        center: GLCoord4D,
        yaw: f32,
    },
    Event(Event),
    CreateDrawing(Drawing),
    UpdateVertices {
        name: String,
        floats: Vec<f32>,
        index: usize,
    },
    UpdateTexture {
        name: String,
        texture: Option<String>,
    },
    SetDrawingVisibility {
        name: String,
        visible: bool,
    },
    Erase(String),
    LookAt(Option<WorldCoord>),
}

pub struct IsometricEngine {
    events_loop: glutin::EventsLoop,
    window: glutin::GlWindow,
    graphics: GraphicsEngine,
    running: bool,
    cursor_handler: CursorHandler,
    event_consumers: Vec<Box<dyn EventConsumer>>,
    command_tx: Sender<Vec<Command>>,
    command_rx: Receiver<Vec<Command>>,
    look_at: Option<WorldCoord>,
}

impl IsometricEngine {
    const GL_VERSION: glutin::GlRequest = glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3));

    pub fn new(title: &str, width: u32, height: u32, max_z: f32) -> IsometricEngine {
        let events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_title(title)
            .with_dimensions(glutin::dpi::LogicalSize::new(
                f64::from(width),
                f64::from(height),
            ));
        let context = glutin::ContextBuilder::new()
            .with_gl(IsometricEngine::GL_VERSION)
            .with_vsync(true)
            .with_multisampling(4);
        let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

        unsafe {
            gl_window.make_current().unwrap();
            gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);
        }

        let dpi_factor = gl_window.get_hidpi_factor();
        let logical_window_size = gl_window.window().get_inner_size().unwrap();
        let graphics =
            GraphicsEngine::new(1.0 / max_z, logical_window_size.to_physical(dpi_factor));

        let (command_tx, command_rx) = mpsc::channel();

        let mut out = IsometricEngine {
            events_loop,
            cursor_handler: CursorHandler::new(dpi_factor, logical_window_size),
            event_consumers: vec![],
            window: gl_window,
            graphics,
            running: true,
            command_rx,
            command_tx,
            look_at: None,
        };

        out.init_event_handlers();

        out
    }

    pub fn command_tx(&self) -> Sender<Vec<Command>> {
        self.command_tx.clone()
    }

    pub fn add_event_consumer<T>(&mut self, event_consumer: T)
    where
        T: EventConsumer + 'static,
    {
        self.event_consumers.push(Box::new(event_consumer));
    }

    pub fn add_event_handler<T>(&mut self, event_handler: T)
    where
        T: EventHandler + 'static,
    {
        let event_consumer = EventHandlerAdapter {
            event_handler: Box::new(event_handler),
            command_tx: self.command_tx.clone(),
        };
        self.add_event_consumer(event_consumer);
    }

    fn init_event_handlers(&mut self) {
        let dpi_factor = self.window.get_hidpi_factor();

        self.add_event_handler(ShutdownHandler::default());
        self.add_event_handler(DPIRelay::default());
        self.add_event_handler(Resizer::default());
        self.add_event_handler(DragHandler::default());
        self.add_event_handler(ResizeRelay::new(dpi_factor));
        self.add_event_handler(Scroller::default());
        self.add_event_handler(KeyRelay::default());
        self.add_event_handler(MouseRelay::default());
    }

    pub fn run(&mut self) {
        while self.running {
            self.handle_commands();
            self.consume_cursors();
            self.consume_glutin_events();
            self.consume_event(Event::Tick);
            self.look_at();
            self.graphics.draw_world();
            self.graphics.draw_textured();
            self.update_cursors();
            self.graphics.draw_ui();
            self.graphics.draw_billboards();
            self.window.swap_buffers().unwrap();
        }

        self.shutdown();
    }

    fn consume_glutin_events(&mut self) {
        let mut glutin_events = vec![];
        self.events_loop.poll_events(|event| {
            glutin_events.push(event);
        });
        for event in glutin_events {
            self.consume_event(Event::GlutinEvent(event));
        }
    }

    fn consume_event(&mut self, event: Event) {
        let event_arc = Arc::new(event);
        self.cursor_handler.consume_event(event_arc.clone());
        for handler in self.event_consumers.iter_mut() {
            handler.consume_event(event_arc.clone());
        }
    }

    fn get_commands(&mut self) -> Vec<Command> {
        let mut out = vec![];
        loop {
            match &mut self.command_rx.try_recv() {
                Ok(commands) => out.append(commands),
                Err(TryRecvError::Empty) => return out,
                Err(TryRecvError::Disconnected) => {
                    panic!("Isometric engine command receiver lost connection!");
                }
            };
        }
    }

    fn handle_commands(&mut self) {
        for command in self.get_commands() {
            self.handle_command(command)
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Shutdown => self.running = false,
            Command::Resize(physical_size) => {
                self.window.resize(physical_size);
                self.graphics.set_viewport_size(physical_size);
            }
            Command::Translate(translation) => self.graphics.transform().translate(translation),
            Command::Scale { center, scale } => self.graphics.transform().scale(center, scale),
            Command::Rotate { center, yaw } => self.graphics.rotate(center, yaw),
            Command::Event(event) => self.consume_event(event),
            Command::CreateDrawing(drawing) => self.graphics.add_drawing(drawing),
            Command::UpdateTexture { name, texture } => self.graphics.update_texture(name, texture),
            Command::UpdateVertices {
                name,
                index,
                floats,
            } => self.graphics.update_vertices(name, index, floats),
            Command::SetDrawingVisibility { name, visible } => {
                self.graphics.set_drawing_visibility(name, visible)
            }
            Command::Erase(name) => self.graphics.remove_drawing(&name),
            Command::LookAt(look_at) => self.look_at = look_at,
        }
    }

    fn look_at(&mut self) {
        if let Some(look_at) = self.look_at {
            self.graphics.transform().look_at(look_at);
        }
    }

    fn update_cursors(&mut self) {
        self.cursor_handler
            .update_gl_and_world_cursor(&mut self.graphics.transform());
    }

    fn consume_cursors(&mut self) {
        self.cursor_handler
            .gl_cursor()
            .iter()
            .for_each(|gl_cursor| self.consume_event(Event::CursorMoved(*gl_cursor)));
        self.cursor_handler
            .world_cursor()
            .iter()
            .for_each(|world_cursor| {
                self.consume_event(Event::WorldPositionChanged(*world_cursor))
            });
    }

    fn shutdown(&mut self) {
        for handler in &mut self.event_consumers {
            handler.consume_event(Arc::new(Event::Shutdown));
        }
    }
}
