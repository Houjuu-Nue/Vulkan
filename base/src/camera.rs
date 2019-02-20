
use winit::VirtualKeyCode;

use crate::input::InputController;

type Point3F  = nalgebra::Point3<f32>;
type Vector3F = nalgebra::Vector3<f32>;
type Matrix4F = nalgebra::Matrix4<f32>;

pub struct FlightCamera {

    /// Camera position.
    pos  : Point3F,
    /// Front direction.
    front: Vector3F,
    /// Up direction.
    up   : Vector3F,
    /// right direction.
    right: Vector3F,

    world_up: Vector3F,

    yaw  : f32,
    pitch: f32,

    // camera options
    move_speed: f32,
    _mouse_sentivity: f32,
    _wheel_sentivity: f32,

    zoom: f32,
    near: f32,
    far : f32,
    screen_aspect: f32,
}

impl FlightCamera {

    pub fn new() -> FlightCameraBuilder {
        FlightCameraBuilder::default()
    }

    pub fn set_move_speed(&mut self, speed: f32) {
        self.move_speed = speed;
    }

    pub fn current_position(&self) -> Point3F {
        self.pos.clone()
    }

    pub fn view_matrix(&self) -> Matrix4F {

        Matrix4F::look_at_rh(&self.pos, &(self.pos + self.front), &self.up)
    }

    pub fn proj_matrix(&self) -> Matrix4F {

        Matrix4F::new_perspective(self.screen_aspect, self.zoom, self.near, self.far)
    }

    pub fn reset_screen_dimension(&mut self, width: u32, height: u32) {
        self.screen_aspect = (width as f32) / (height as f32);
    }

    pub fn receive_input(&mut self, inputer: &InputController, delta_time: f32) {

        // keyboard
        let velocity = self.move_speed * delta_time;

        if inputer.key.is_key_pressed(VirtualKeyCode::Up) {
            self.pos += self.front * velocity;
        } else if inputer.key.is_key_pressed(VirtualKeyCode::Down) {
            self.pos -= self.front * velocity;
        }

        if inputer.key.is_key_pressed(VirtualKeyCode::Left) {
            self.pos -= self.right * velocity;
        } else if inputer.key.is_key_pressed(VirtualKeyCode::Right) {
            self.pos += self.right * velocity;
        }

        // mouse motion
        if inputer.is_cursor_active() {

            let mouse_motion = inputer.cursor.get_cursor_motion();

            self.yaw += mouse_motion.0;
            self.pitch = num::clamp(self.pitch - mouse_motion.1, -89.0, 89.0);

            // recalculate front, right or up vector only when mouse move.
            self.update_vectors();
        }
    }

    fn update_vectors(&mut self) {
        // calculate the new front vector.
        let front_x = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
        let front_y = self.pitch.to_radians().sin();
        let front_z = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();

        self.front = Vector3F::new(front_x, front_y, front_z).normalize();

        // also calculate the right and up vector.
        // Normalize the vectors, because their length gets closer to 0 the move you look up or down which results in slower movement.
        self.right = self.front.cross(&self.world_up);
        self.up    = self.right.cross(&self.front);
    }
}

pub struct FlightCameraBuilder {

    pos     : Point3F,
    world_up: Vector3F,

    yaw  : f32,
    pitch: f32,

    near: f32,
    far : f32,
    screen_aspect: f32,
}

impl Default for FlightCameraBuilder {

    fn default() -> FlightCameraBuilder {
        FlightCameraBuilder {
            pos      : Point3F::new(0.0, 0.0, 0.0),
            world_up : Vector3F::y(),
            yaw      : -90.0,
            pitch    : 0.0,
            near     : 0.1,
            far      : 100.0,
            screen_aspect: 1.0,
        }
    }
}

impl FlightCameraBuilder {

    pub fn build(self) -> FlightCamera {
        FlightCamera {
            pos      : self.pos,
            front    : Vector3F::new(0.0, 0.0, -1.0),
            up       : nalgebra::zero(),
            right    : nalgebra::zero(),
            world_up : self.world_up,
            yaw      : self.yaw,
            pitch    : self.pitch,
            near     : self.near,
            far      : self.far,
            screen_aspect: self.screen_aspect,

            move_speed: 2.5,
            _mouse_sentivity: 1.0,
            _wheel_sentivity: 1.0,
            zoom: 45.0
        }
    }

    pub fn place_at(mut self, position: Point3F) -> FlightCameraBuilder {
        self.pos = position; self
    }

    pub fn world_up(mut self, up: Vector3F) -> FlightCameraBuilder {
        self.world_up = up;
        self
    }

    pub fn yaw(mut self, yaw: f32) -> FlightCameraBuilder {
        self.yaw = yaw; self
    }

    pub fn pitch(mut self, pitch: f32) -> FlightCameraBuilder {
        self.pitch = pitch; self
    }

    pub fn view_distance(mut self, near: f32, far: f32) -> FlightCameraBuilder {
        self.near = near;
        self.far = far;self
    }

    pub fn screen_aspect_ratio(mut self, ratio: f32) -> FlightCameraBuilder {
        self.screen_aspect = ratio; self
    }
}