
use super::app_control::AppControl;
use super::input_state::InputState;
use super::winit_input_handler::WinitInputHandler;
use super::time_state::TimeState;
use super::time_state::TimeContext;
use super::util::PeriodicEvent;
use std::ffi::CString;

use crate::RendererBuilder;

pub trait AppHandler {
    fn update(
        &mut self,
        app_control: &mut AppControl,
        input_state: &InputState,
        time_state: &TimeState
    );

    fn draw(
        &mut self,
        app_control: &AppControl,
        input_state: &InputState,
        time_state: &TimeState,
        canvas: &mut skia_safe::Canvas,
    );
}

pub struct AppBuilder {
    app_name: CString,
    use_vulkan_debug_layer: bool
}

impl AppBuilder {
    pub fn new() -> Self {
        AppBuilder {
            app_name: CString::new("Skulpin").unwrap(),
            use_vulkan_debug_layer: false
        }
    }

    pub fn app_name(mut self, app_name: CString) -> Self {
        self.app_name = app_name;
        self
    }

    pub fn use_vulkan_debug_layer(mut self, use_vulkan_debug_layer: bool) -> Self {
        self.use_vulkan_debug_layer = use_vulkan_debug_layer;
        self
    }

    pub fn run<T : 'static + AppHandler>(&self, app_handler: T) -> Result<(), Box<dyn std::error::Error>> {
        App::run(app_handler, &self.app_name, self.use_vulkan_debug_layer)
    }
}

pub struct App {

}

impl App {
    //TODO: Since winit returns !, we should just take a callback here for handling errors instead
    // of returning
    pub fn run<T : 'static + AppHandler>(
        mut app_handler: T,
        app_name: &CString,
        use_vulkan_debug_layer: bool
    )
        -> Result<(), Box<dyn std::error::Error>>
    {
        // Create the event loop
        let event_loop = winit::event_loop::EventLoop::<()>::with_user_event();

        // Create a single window
        let window = winit::window::WindowBuilder::new()
            .with_title("Skulpin")
            .with_inner_size(winit::dpi::LogicalSize::new(1300.0, 900.0))
            .build(&event_loop)?;

        let mut app_control = AppControl::default();
        let mut input_state = InputState::default();
        let mut input_handler = WinitInputHandler::new();
        let mut time_state = TimeState::default();

        let mut renderer = RendererBuilder::new()
            .use_vulkan_debug_layer(use_vulkan_debug_layer)
            .app_name(app_name.clone())
            .build(&window)?;

        // To print fps once per second
        let mut print_fps_event = PeriodicEvent::default();

        // Pass control of this thread to winit until the app terminates. If this app wants to quit,
        // the update loop should send the appropriate event via the channel
        event_loop.run(move |event, window_target, control_flow| {
            match event {
                winit::event::Event::EventsCleared => {

                    time_state.update(TimeContext::System);

                    if print_fps_event.try_take_event(
                        time_state.system().frame_start_instant,
                        std::time::Duration::from_secs_f32(1.0)
                    ) {
                        info!("fps: {}", time_state.system().fps);
                    }

                    app_handler.update(
                        &mut app_control,
                        &input_state,
                        &time_state
                    );

                    // Call this to mark the start of the next frame (i.e. "key just down" will return false)
                    input_state.end_frame();

                    // Queue a RedrawRequested event.
                    window.request_redraw();
                },
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::RedrawRequested,
                    ..
                } => {
                    if let Err(e) = renderer.draw(&window, |canvas| {
                        app_handler.draw(
                            &app_control,
                            &input_state,
                            &time_state,
                            canvas
                        );
                    }) {
                        //TODO: Handle Error
                        warn!("{:?}", e);
                        app_control.enqueue_terminate_process();
                    }
                },
                _ => input_handler.handle_input(
                    &mut app_control,
                    &mut input_state,
                    event,
                    window_target)
            }

            if app_control.should_terminate_process() {
                *control_flow = winit::event_loop::ControlFlow::Exit
            }
        });
    }
}