use std::sync::Arc;

use commons::async_channel::{unbounded, Receiver, Sender, TryRecvError};
use commons::log::debug;
use coords::*;
use cursor_handler::*;
use event_handlers::*;
use events::{EventConsumer, EventHandler, EventHandlerAdapter};
use glutin::event_loop::ControlFlow;
use glutin::platform::run_return::EventLoopExtRunReturn;
use glutin::{PossiblyCurrent, WindowedContext};
use graphics::{Drawing, GraphicsEngine, GraphicsEngineParameters};

#[derive(Debug, PartialEq)]
pub enum Button {
    Key(glutin::event::VirtualKeyCode),
    Mouse(glutin::event::MouseButton),
}

#[derive(Debug)]
pub enum Event {
    Start,
    Tick,
    Shutdown,
    CursorMoved(Option<GLCoord4D>),
    WorldPositionChanged(Option<WorldCoord>),
    GlutinEvent(glutin::event::Event<'static, ()>),
    Drag(GLCoord4D),
    Button {
        button: Button,
        state: glutin::event::ElementState,
        modifiers: glutin::event::ModifiersState,
    },
}

#[derive(Debug)]
pub enum Command {
    Shutdown,
    Resize(glutin::dpi::PhysicalSize<u32>),
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
    UpdateMask {
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
    events_loop: glutin::event_loop::EventLoop<()>,
    windowed_context: WindowedContext<PossiblyCurrent>,
    graphics: GraphicsEngine,
    running: bool,
    cursor_handler: CursorHandler,
    event_consumers: Vec<Box<dyn EventConsumer>>,
    command_tx: Sender<Vec<Command>>,
    command_rx: Receiver<Vec<Command>>,
    look_at: Option<WorldCoord>,
}

pub struct IsometricEngineParameters<'a> {
    pub title: &'a str,
    pub width: u32,
    pub height: u32,
    pub max_z: f32,
    pub label_padding: f32,
}

impl IsometricEngine {
    const GL_VERSION: glutin::GlRequest = glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3));

    pub fn new(params: IsometricEngineParameters) -> IsometricEngine {
        let events_loop = glutin::event_loop::EventLoop::new();
        let window = glutin::window::WindowBuilder::new()
            .with_title(params.title)
            .with_inner_size(glutin::dpi::PhysicalSize::new(
                f64::from(params.width),
                f64::from(params.height),
            ));
        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(IsometricEngine::GL_VERSION)
            .with_vsync(true)
            .with_multisampling(4)
            .build_windowed(window, &events_loop)
            .unwrap();
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };

        gl::load_with(|symbol| windowed_context.get_proc_address(symbol) as *const _);

        let physical_window_size = windowed_context.window().inner_size();
        let graphics = GraphicsEngine::new(GraphicsEngineParameters {
            z_scale: 1.0 / params.max_z,
            viewport_size: physical_window_size,
            label_padding: params.label_padding,
        });

        let (command_tx, command_rx) = unbounded();

        let mut out = IsometricEngine {
            events_loop,
            cursor_handler: CursorHandler::new(physical_window_size),
            event_consumers: vec![],
            windowed_context,
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
        self.add_event_handler(ShutdownHandler::default());
        self.add_event_handler(Resizer::default());
        self.add_event_handler(DragHandler::default());
        self.add_event_handler(Scroller::default());
        self.add_event_handler(KeyRelay::default());
        self.add_event_handler(MouseRelay::default());
    }

    pub fn run(&mut self) {
        self.graphics.bind();
        while self.running {
            let start = std::time::Instant::now();
            self.handle_commands();
            self.consume_cursors();
            self.consume_glutin_events();
            self.consume_event(Event::Tick);
            self.look_at();
            self.graphics.begin_drawing();
            self.graphics.draw_world();
            self.graphics.draw_textured();
            self.graphics.draw_billboards();
            self.graphics.copy_to_back_buffer();
            self.graphics.draw_ui();
            self.graphics.bind();
            let before_update_cursors = start.elapsed().as_micros();
            self.update_cursors();
            let after_update_cursors = start.elapsed().as_micros();
            self.windowed_context.swap_buffers().unwrap();
            if start.elapsed().as_micros() > 17000 {

                debug!("{}/{}/{}", 
                before_update_cursors,
                after_update_cursors,
                start.elapsed().as_micros());
            }
        }

        self.shutdown();
    }

    fn consume_glutin_events(&mut self) {
        let mut glutin_events = vec![];
        self.events_loop.run_return(|event, _, control_flow| {
            if let Some(event) = event.to_static() {
                glutin_events.push(event);
            }
            *control_flow = ControlFlow::Exit
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
                Err(TryRecvError::Closed) => {
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
                self.windowed_context.resize(physical_size);
                self.graphics.set_viewport_size(physical_size);
                self.graphics.setup_frame_buffer();
            }
            Command::Translate(translation) => self.graphics.transform().translate(translation),
            Command::Scale { center, scale } => self.graphics.transform().scale(center, scale),
            Command::Rotate { center, yaw } => self.graphics.rotate(center, yaw),
            Command::Event(event) => self.consume_event(event),
            Command::CreateDrawing(drawing) => self.graphics.add_drawing(drawing),
            Command::UpdateTexture { name, texture } => self.graphics.update_texture(name, texture),
            Command::UpdateMask { name, texture } => self.graphics.update_mask(name, texture),
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
        let gl_cursor = self.cursor_handler.gl_cursor();
        self.consume_event(Event::CursorMoved(gl_cursor));
        let world_cursor = self.cursor_handler.world_cursor();
        self.consume_event(Event::WorldPositionChanged(world_cursor));
    }

    fn shutdown(&mut self) {
        for handler in &mut self.event_consumers {
            handler.consume_event(Arc::new(Event::Shutdown));
        }
    }
}
