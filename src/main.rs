mod application;
mod boid;
mod camera;
// mod camera;

use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::Window;
use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};
use crate::application::{ApplicationState, SimulationParams};


async fn run(event_loop: EventLoop<()>, window:Window){
    // Creating the application
    let mut app = ApplicationState::init(&window, SimulationParams{
        separation_reach: 4.0,
        separation_scale: 1.0,
        alignement_reach: 1.0,
        alignement_scale: 3.5,
        cohesion_reach: 4.0,
        cohesion_scale: 3.0,
        color_mult: 1.0,
        step_mult: 1.0,
        center_attraction: 0.05,
    }).await;
    
    
    event_loop.run( move | event, _, control_flow|{
        match event {
            // Only handle window event
            Event::WindowEvent {event, window_id,..}if window_id == window.id()  => {
                match event {
                    // Input handled by application so do nothing
                    event if app.input(&event) => {}
                    // Stop the loop if the application is required to stop
                    WindowEvent::CloseRequested |
                    WindowEvent::Destroyed |
                    WindowEvent::KeyboardInput { input:KeyboardInput {state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. }, ..} => {
                        *control_flow = ControlFlow::Exit;
                    }
                    // Handle resizing
                    WindowEvent::Resized(size) =>{
                        app.resize(size);
                    }
                    WindowEvent::ScaleFactorChanged{new_inner_size, ..} =>{
                        app.resize(*new_inner_size);
                    }
                    _ => {}
                }
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                app.update();
                match app.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => app.resize(app.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            },
            Event::RedrawEventsCleared | Event::MainEventsCleared => {
                window.request_redraw();
            }
            // Any other event is ignore
            _ => {}
        }
    });
}

fn main() {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
