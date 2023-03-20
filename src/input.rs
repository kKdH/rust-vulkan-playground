use bitflags::bitflags;
use nalgebra::Vector3;
use winit::event::{ElementState, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent};
use crate::movable::Movable;


pub struct KeyboardMovementController {
    movement_flags: MovementBitFlags,
    rotation: Vector3::<f32>,
    rotation_speed: f32,
    translation: Vector3::<f32>,
    translation_speed: f32,
}

impl KeyboardMovementController {

    pub fn new(rotation_speed: f32, translation_speed: f32) -> Self {

        KeyboardMovementController {
            movement_flags: MovementBitFlags::NONE,
            rotation: Vector3::zeros(),
            rotation_speed,
            translation: Vector3::zeros(),
            translation_speed,
        }
    }

    pub fn handle(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::W),
                    ..
                } => {
                    self.set_active(&Movement::MoveForward, ElementState::Pressed == *state)
                },
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::A),
                    ..
                } => {
                    self.set_active(&Movement::StrafeLeft, ElementState::Pressed == *state)
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::S),
                    ..
                } => {
                    self.set_active(&Movement::MoveBackward, ElementState::Pressed == *state)
                },
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::D),
                    ..
                } => {
                    self.set_active(&Movement::StrafeRight, ElementState::Pressed == *state)
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Up),
                    ..
                } => {
                    self.set_active(&Movement::LookUp, ElementState::Pressed == *state)
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Down),
                    ..
                } => {
                    self.set_active(&Movement::LookDown, ElementState::Pressed == *state)
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Left),
                    ..
                } => {
                    self.set_active(&Movement::LookLeft, ElementState::Pressed == *state)
                }
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Right),
                    ..
                } => {
                    self.set_active(&Movement::LookRight, ElementState::Pressed == *state)
                }
                &_ => {}
            }
            WindowEvent::ModifiersChanged(state) => {
                self.set_active(&Movement::FastMovement, state.shift());
            }
            _ => {}
        };
    }

    pub fn apply(&mut self, dt: f32, movable: &mut Movable) {

        if self.movement_flags != MovementBitFlags::NONE {

            self.rotation.fill(0.0);
            self.translation.fill(0.0);

            if self.is_active(&Movement::LookLeft) { self.rotation.y -= 1.0 }
            if self.is_active(&Movement::LookRight) { self.rotation.y += 1.0 }
            if self.is_active(&Movement::LookUp) { self.rotation.x += 1.0 }
            if self.is_active(&Movement::LookDown) { self.rotation.x -= 1.0 }

            if self.rotation.dot(&self.rotation) > f32::EPSILON {
                let speed = if self.is_active(&Movement::FastMovement) {
                    self.rotation_speed * 4.0
                }
                else {
                    self.rotation_speed
                };
                self.rotation.normalize_mut();
                self.rotation.scale_mut(speed * dt);
                movable.rotate(&self.rotation);
            }

            if self.is_active(&Movement::MoveForward) { self.translation += movable.forward() }
            if self.is_active(&Movement::MoveBackward) { self.translation -= movable.forward() }
            if self.is_active(&Movement::StrafeLeft) { self.translation -= movable.right() }
            if self.is_active(&Movement::StrafeRight) { self.translation += movable.right() }

            if self.translation.dot(&self.translation) > f32::EPSILON {
                let speed = if self.is_active(&Movement::FastMovement) {
                    self.translation_speed * 4.0
                }
                else {
                    self.translation_speed
                };
                self.translation.normalize_mut();
                self.translation.scale_mut(speed * dt);
                movable.translate(&self.translation);
            }
        }
    }

    fn set_active(&mut self, movement: &Movement, active: bool) {
        match (movement, active) {
            (Movement::MoveForward, true) => self.movement_flags.insert(MovementBitFlags::MOVE_FORWARD),
            (Movement::MoveForward, false) => self.movement_flags.remove(MovementBitFlags::MOVE_FORWARD),
            (Movement::MoveBackward, true) => self.movement_flags.insert(MovementBitFlags::MOVE_BACKWARD),
            (Movement::MoveBackward, false) => self.movement_flags.remove(MovementBitFlags::MOVE_BACKWARD),
            (Movement::StrafeLeft, true) => self.movement_flags.insert(MovementBitFlags::STRAFE_LEFT),
            (Movement::StrafeLeft, false) => self.movement_flags.remove(MovementBitFlags::STRAFE_LEFT),
            (Movement::StrafeRight, true) => self.movement_flags.insert(MovementBitFlags::STRAFE_RIGHT),
            (Movement::StrafeRight, false) => self.movement_flags.remove(MovementBitFlags::STRAFE_RIGHT),
            (Movement::LookLeft, true) => self.movement_flags.insert(MovementBitFlags::LOOK_LEFT),
            (Movement::LookLeft, false) => self.movement_flags.remove(MovementBitFlags::LOOK_LEFT),
            (Movement::LookRight, true) => self.movement_flags.insert(MovementBitFlags::LOOK_RIGHT),
            (Movement::LookRight, false) => self.movement_flags.remove(MovementBitFlags::LOOK_RIGHT),
            (Movement::LookUp, true) => self.movement_flags.insert(MovementBitFlags::LOOK_UP),
            (Movement::LookUp, false) => self.movement_flags.remove(MovementBitFlags::LOOK_UP),
            (Movement::LookDown, true) => self.movement_flags.insert(MovementBitFlags::LOOK_DOWN),
            (Movement::LookDown, false) => self.movement_flags.remove(MovementBitFlags::LOOK_DOWN),
            (Movement::FastMovement, true) => self.movement_flags.insert(MovementBitFlags::FAST_MOVEMENT),
            (Movement::FastMovement, false) => self.movement_flags.remove(MovementBitFlags::FAST_MOVEMENT),
        };
    }

    fn is_active(&self, movement: &Movement) -> bool {
        match movement {
            Movement::MoveForward => self.movement_flags.contains(MovementBitFlags::MOVE_FORWARD),
            Movement::MoveBackward => self.movement_flags.contains(MovementBitFlags::MOVE_BACKWARD),
            Movement::StrafeLeft => self.movement_flags.contains(MovementBitFlags::STRAFE_LEFT),
            Movement::StrafeRight => self.movement_flags.contains(MovementBitFlags::STRAFE_RIGHT),
            Movement::LookLeft => self.movement_flags.contains(MovementBitFlags::LOOK_LEFT),
            Movement::LookRight => self.movement_flags.contains(MovementBitFlags::LOOK_RIGHT),
            Movement::LookUp => self.movement_flags.contains(MovementBitFlags::LOOK_UP),
            Movement::LookDown => self.movement_flags.contains(MovementBitFlags::LOOK_DOWN),
            Movement::FastMovement => self.movement_flags.contains(MovementBitFlags::FAST_MOVEMENT),
        }
    }
}

enum Movement {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    LookLeft,
    LookRight,
    LookUp,
    LookDown,
    FastMovement,
}

bitflags! {
    struct MovementBitFlags: u64 {
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
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use crate::input::MovementBitFlags;

    #[test]
    fn test_MovementBitFlags() {

        let forward_left = MovementBitFlags::MOVE_FORWARD | MovementBitFlags::STRAFE_LEFT;

        assert_eq!((forward_left - MovementBitFlags::STRAFE_LEFT), MovementBitFlags::MOVE_FORWARD);
        assert_eq!((forward_left - MovementBitFlags::MOVE_FORWARD), MovementBitFlags::STRAFE_LEFT);
        assert_eq!((forward_left - (MovementBitFlags::MOVE_FORWARD | MovementBitFlags::STRAFE_LEFT)), MovementBitFlags::NONE);
    }
}
