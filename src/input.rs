use bitflags::bitflags;
use nalgebra::{Vector2, Vector3};
use winit::event::{ElementState, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent};
use crate::movable::Movable;

const MOUSE_ACCELERATION_FACTOR: f32 = 0.01;

pub struct MovementController {
    key_state_flags: KeyStateFlags,
    rotation: Vector3::<f32>,
    rotation_speed: f32,
    translation: Vector3::<f32>,
    translation_speed: f32,
    cursor_position_delta: Vector2<f32>,
    last_cursor_position: Vector2<f32>,
    mouse_acceleration: Vector2<f32>,
}

impl MovementController {

    pub fn new(rotation_speed: f32, translation_speed: f32, mouse_acceleration: Vector2<f32>) -> Self {

        MovementController {
            key_state_flags: KeyStateFlags::NONE,
            rotation: Vector3::zeros(),
            rotation_speed,
            translation: Vector3::zeros(),
            translation_speed,
            cursor_position_delta: Vector2::zeros(),
            last_cursor_position: Vector2::zeros(),
            mouse_acceleration,
        }
    }

    pub fn handle(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;
                self.cursor_position_delta.x = x - self.last_cursor_position.x;
                self.cursor_position_delta.y = y - self.last_cursor_position.y;
                self.last_cursor_position.x = x;
                self.last_cursor_position.y = y;
            }
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::W),
                    ..
                } => {
                    self.set_active(&KeyAction::MoveForward, ElementState::Pressed == *state);
                },
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::A),
                    ..
                } => {
                    self.set_active(&KeyAction::StrafeLeft, ElementState::Pressed == *state);
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::S),
                    ..
                } => {
                    self.set_active(&KeyAction::MoveBackward, ElementState::Pressed == *state);
                },
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::D),
                    ..
                } => {
                    self.set_active(&KeyAction::StrafeRight, ElementState::Pressed == *state);
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Up),
                    ..
                } => {
                    self.set_active(&KeyAction::LookUp, ElementState::Pressed == *state);
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Down),
                    ..
                } => {
                    self.set_active(&KeyAction::LookDown, ElementState::Pressed == *state);
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Left),
                    ..
                } => {
                    self.set_active(&KeyAction::LookLeft, ElementState::Pressed == *state);
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Right),
                    ..
                } => {
                    self.set_active(&KeyAction::LookRight, ElementState::Pressed == *state);
                }
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::Space),
                    ..
                } => {
                    self.toggle_active(&KeyAction::MouseLook);
                }
                &_ => {}
            }
            WindowEvent::ModifiersChanged(state) => {
                self.set_active(&KeyAction::FastMovement, state.shift());
            }
            _ => {}
        };
    }

    pub fn apply(&mut self, dt: f32, movable: &mut Movable) {

        if self.key_state_flags != KeyStateFlags::NONE {

            self.rotation.fill(0.0);
            self.translation.fill(0.0);

            let (rotation_speed, translation_speed) = if self.is_active(&KeyAction::FastMovement) {
                (self.rotation_speed * 4.0, self.translation_speed * 4.0)
            }
            else {
                (self.rotation_speed, self.translation_speed)
            };

            if self.is_active(&KeyAction::LookLeft) { self.rotation.y -= rotation_speed }
            if self.is_active(&KeyAction::LookRight) { self.rotation.y += rotation_speed }
            if self.is_active(&KeyAction::LookUp) { self.rotation.x += rotation_speed }
            if self.is_active(&KeyAction::LookDown) { self.rotation.x -= rotation_speed }

            if self.is_active(&KeyAction::MouseLook) {
                self.rotation.y -= self.cursor_position_delta.x * self.mouse_acceleration.y * MOUSE_ACCELERATION_FACTOR;
                self.rotation.x += self.cursor_position_delta.y * self.mouse_acceleration.x * MOUSE_ACCELERATION_FACTOR;
                self.cursor_position_delta.fill(0.0);
            }

            if self.rotation.dot(&self.rotation) > f32::EPSILON {
                self.rotation.scale_mut(dt);
                movable.rotate(&self.rotation);
            }

            if self.is_active(&KeyAction::MoveForward) { self.translation += movable.forward() }
            if self.is_active(&KeyAction::MoveBackward) { self.translation -= movable.forward() }
            if self.is_active(&KeyAction::StrafeLeft) { self.translation -= movable.right() }
            if self.is_active(&KeyAction::StrafeRight) { self.translation += movable.right() }

            if self.translation.dot(&self.translation) > f32::EPSILON {
                self.translation.normalize_mut();
                self.translation.scale_mut(translation_speed * dt);
                movable.translate(&self.translation);
            }
        }
    }

    fn set_active(&mut self, movement: &KeyAction, active: bool) {
        match (movement, active) {
            (KeyAction::MoveForward, true) => self.key_state_flags.insert(KeyStateFlags::MOVE_FORWARD),
            (KeyAction::MoveForward, false) => self.key_state_flags.remove(KeyStateFlags::MOVE_FORWARD),
            (KeyAction::MoveBackward, true) => self.key_state_flags.insert(KeyStateFlags::MOVE_BACKWARD),
            (KeyAction::MoveBackward, false) => self.key_state_flags.remove(KeyStateFlags::MOVE_BACKWARD),
            (KeyAction::StrafeLeft, true) => self.key_state_flags.insert(KeyStateFlags::STRAFE_LEFT),
            (KeyAction::StrafeLeft, false) => self.key_state_flags.remove(KeyStateFlags::STRAFE_LEFT),
            (KeyAction::StrafeRight, true) => self.key_state_flags.insert(KeyStateFlags::STRAFE_RIGHT),
            (KeyAction::StrafeRight, false) => self.key_state_flags.remove(KeyStateFlags::STRAFE_RIGHT),
            (KeyAction::LookLeft, true) => self.key_state_flags.insert(KeyStateFlags::LOOK_LEFT),
            (KeyAction::LookLeft, false) => self.key_state_flags.remove(KeyStateFlags::LOOK_LEFT),
            (KeyAction::LookRight, true) => self.key_state_flags.insert(KeyStateFlags::LOOK_RIGHT),
            (KeyAction::LookRight, false) => self.key_state_flags.remove(KeyStateFlags::LOOK_RIGHT),
            (KeyAction::LookUp, true) => self.key_state_flags.insert(KeyStateFlags::LOOK_UP),
            (KeyAction::LookUp, false) => self.key_state_flags.remove(KeyStateFlags::LOOK_UP),
            (KeyAction::LookDown, true) => self.key_state_flags.insert(KeyStateFlags::LOOK_DOWN),
            (KeyAction::LookDown, false) => self.key_state_flags.remove(KeyStateFlags::LOOK_DOWN),
            (KeyAction::FastMovement, true) => self.key_state_flags.insert(KeyStateFlags::FAST_MOVEMENT),
            (KeyAction::FastMovement, false) => self.key_state_flags.remove(KeyStateFlags::FAST_MOVEMENT),
            (KeyAction::MouseLook, true) => self.key_state_flags.insert(KeyStateFlags::MOUSE_LOOK),
            (KeyAction::MouseLook, false) => self.key_state_flags.remove(KeyStateFlags::MOUSE_LOOK),
        };
    }

    fn toggle_active(&mut self, action: &KeyAction) {
        match action {
            KeyAction::MoveForward => self.key_state_flags.toggle(KeyStateFlags::MOVE_FORWARD),
            KeyAction::MoveBackward => self.key_state_flags.toggle(KeyStateFlags::MOVE_BACKWARD),
            KeyAction::StrafeLeft => self.key_state_flags.toggle(KeyStateFlags::STRAFE_LEFT),
            KeyAction::StrafeRight => self.key_state_flags.toggle(KeyStateFlags::STRAFE_RIGHT),
            KeyAction::LookLeft => self.key_state_flags.toggle(KeyStateFlags::LOOK_LEFT),
            KeyAction::LookRight => self.key_state_flags.toggle(KeyStateFlags::LOOK_RIGHT),
            KeyAction::LookUp => self.key_state_flags.toggle(KeyStateFlags::LOOK_UP),
            KeyAction::LookDown => self.key_state_flags.toggle(KeyStateFlags::LOOK_DOWN),
            KeyAction::FastMovement => self.key_state_flags.toggle(KeyStateFlags::FAST_MOVEMENT),
            KeyAction::MouseLook => self.key_state_flags.toggle(KeyStateFlags::MOUSE_LOOK),
        };
    }

    fn is_active(&self, movement: &KeyAction) -> bool {
        match movement {
            KeyAction::MoveForward => self.key_state_flags.contains(KeyStateFlags::MOVE_FORWARD),
            KeyAction::MoveBackward => self.key_state_flags.contains(KeyStateFlags::MOVE_BACKWARD),
            KeyAction::StrafeLeft => self.key_state_flags.contains(KeyStateFlags::STRAFE_LEFT),
            KeyAction::StrafeRight => self.key_state_flags.contains(KeyStateFlags::STRAFE_RIGHT),
            KeyAction::LookLeft => self.key_state_flags.contains(KeyStateFlags::LOOK_LEFT),
            KeyAction::LookRight => self.key_state_flags.contains(KeyStateFlags::LOOK_RIGHT),
            KeyAction::LookUp => self.key_state_flags.contains(KeyStateFlags::LOOK_UP),
            KeyAction::LookDown => self.key_state_flags.contains(KeyStateFlags::LOOK_DOWN),
            KeyAction::FastMovement => self.key_state_flags.contains(KeyStateFlags::FAST_MOVEMENT),
            KeyAction::MouseLook => self.key_state_flags.contains(KeyStateFlags::MOUSE_LOOK),
        }
    }
}

enum KeyAction {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    LookLeft,
    LookRight,
    LookUp,
    LookDown,
    FastMovement,
    MouseLook,
}

bitflags! {
    struct KeyStateFlags: u64 {
        const NONE          = 0b0000000000000000;
        const MOVE_FORWARD  = 0b0000000000000001;
        const MOVE_BACKWARD = 0b0000000000000010;
        const STRAFE_LEFT   = 0b0000000000000100;
        const STRAFE_RIGHT  = 0b0000000000001000;
        const LOOK_LEFT     = 0b0000000000010000;
        const LOOK_RIGHT    = 0b0000000000100000;
        const LOOK_UP       = 0b0000000001000000;
        const LOOK_DOWN     = 0b0000000010000000;
        const FAST_MOVEMENT = 0b0000000100000000;
        const MOUSE_LOOK    = 0b0000001000000000;
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use crate::input::KeyStateFlags;

    #[test]
    fn test_MovementBitFlags() {

        let forward_left = KeyStateFlags::MOVE_FORWARD | KeyStateFlags::STRAFE_LEFT;

        assert_eq!((forward_left - KeyStateFlags::STRAFE_LEFT), KeyStateFlags::MOVE_FORWARD);
        assert_eq!((forward_left - KeyStateFlags::MOVE_FORWARD), KeyStateFlags::STRAFE_LEFT);
        assert_eq!((forward_left - (KeyStateFlags::MOVE_FORWARD | KeyStateFlags::STRAFE_LEFT)), KeyStateFlags::NONE);
    }
}
