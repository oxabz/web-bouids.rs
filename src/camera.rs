use bytemuck::{Pod, Zeroable};
use winit::event::{WindowEvent, KeyboardInput, VirtualKeyCode, ElementState};

#[derive(Debug)]
pub struct Camera{
    origin:[f32; 2],
    scaling:[f32; 2]
}

impl Camera {
    pub fn new()->Self{
        Self{ origin: [0.0,0.0], scaling: [200.0, 200.0] }
    }

    pub fn build_scaling(&self, size:winit::dpi::PhysicalSize<u32>) -> [f32; 2] {
        return [self.scaling[0] / size.width as f32, self.scaling[1] / size.height as f32]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform{
    origin: [f32; 2],
    scaling: [f32; 2]
}

impl CameraUniform {
    pub fn new() -> Self{
        Self{ origin: [2.0,2.0], scaling: [0.1,0.1] }
    }


    pub fn update_view_proj(&mut self, camera: &Camera, size:winit::dpi::PhysicalSize<u32>) {
        self.scaling = camera.build_scaling(size);
        self.origin = camera.origin;
    }
}

pub struct CameraController {
    move_speed: f32,
    zoom_speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_zoom_pressed: bool,
    is_unzoom_pressed: bool,
}

impl CameraController {
    pub(crate) fn new(move_speed: f32, zoom_speed: f32) -> Self {
        Self {
            move_speed,
            zoom_speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_zoom_pressed: false,
            is_unzoom_pressed: false,
        }
    }

    pub(crate) fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Plus => {
                        self.is_zoom_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Minus => {
                        self.is_unzoom_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub(crate) fn update_camera(&self, camera: &mut Camera) -> bool {
        let mut delta:[f32;2] = [0.0,0.0];
        if self.is_forward_pressed {
            delta[1]+=1.0;
        }
        if self.is_backward_pressed {
            delta[1]-=1.0;
        }
        if self.is_left_pressed {
            delta[0]-=1.0;
        }
        if self.is_right_pressed {
            delta[0]+=1.0;
        }
        let mag = (delta[0] * delta[0] + delta[1]*delta[1]).sqrt();

        if mag > 0.0 {
            camera.origin[0]+= delta[0]/mag*self.move_speed;
            camera.origin[1]+= delta[1]/mag*self.move_speed;
            return true
        }

        if self.is_zoom_pressed && !self.is_unzoom_pressed {
            camera.scaling = [camera.scaling[0]*(1.0-self.zoom_speed), camera.scaling[1]*(1.0-self.zoom_speed)]
        } else if self.is_unzoom_pressed {
            camera.scaling = [camera.scaling[0]*(1.0+self.zoom_speed), camera.scaling[1]*(1.0+self.zoom_speed)]
        }

        false
    }
}
