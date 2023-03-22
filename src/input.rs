use bitflags::bitflags;
use nalgebra::{Vector2, Vector3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::clock::Tick;
use crate::movable::Movable;

pub struct MovementControllerSettings {
    pub rotation_speed: f32,
    pub translation_speed: f32,
    pub mouse_acceleration: Vector2<f32>,
    pub fast_movement_multiplier: f32,
    pub reset_rotation: Vector3<f32>,
    pub reset_translation: Vector3<f32>,
}

pub struct MovementController {
    settings: MovementControllerSettings,
    key_state_flags: KeyStateFlags,
    rotation: Vector3::<f32>,
    translation: Vector3::<f32>,
    cursor_position_delta: Vector2<f32>,
    last_cursor_position: Vector2<f32>,
}

impl MovementController {

    const MOUSE_ACCELERATION_FACTOR: f32 = 0.075;
    const ROTATION_SPEED_FACTOR: f32 = 2.0;
    const TRANSLATION_SPEED_FACTOR: f32 = 2.0;

    pub fn new(settings: MovementControllerSettings) -> Self {

        MovementController {
            settings,
            key_state_flags: KeyStateFlags::NONE,
            rotation: Vector3::zeros(),
            translation: Vector3::zeros(),
            cursor_position_delta: Vector2::zeros(),
            last_cursor_position: Vector2::zeros(),
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
                    virtual_keycode: Some(VirtualKeyCode::R),
                    ..
                } => {
                    self.set_active(&KeyAction::ResetMovement, ElementState::Pressed == *state);
                }
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

    pub fn apply(&mut self, tick: &Tick, movable: &mut Movable) {

        if self.key_state_flags != KeyStateFlags::NONE {

            if self.is_active(&KeyAction::ResetMovement) {
                movable.set_rotation(&self.settings.reset_rotation);
                movable.set_translation(&self.settings.reset_translation);
            }
            else {
                self.rotation.fill(0.0);
                self.translation.fill(0.0);

                let (rotation_speed, translation_speed) = if self.is_active(&KeyAction::FastMovement) {
                    (
                        self.settings.rotation_speed * Self::ROTATION_SPEED_FACTOR * self.settings.fast_movement_multiplier,
                        self.settings.translation_speed * Self::TRANSLATION_SPEED_FACTOR * self.settings.fast_movement_multiplier
                    )
                } else {
                    (
                        self.settings.rotation_speed * Self::ROTATION_SPEED_FACTOR,
                        self.settings.translation_speed * Self::TRANSLATION_SPEED_FACTOR
                    )
                };

                if self.is_active(&KeyAction::LookLeft) { self.rotation.y -= rotation_speed }
                if self.is_active(&KeyAction::LookRight) { self.rotation.y += rotation_speed }
                if self.is_active(&KeyAction::LookUp) { self.rotation.x += rotation_speed }
                if self.is_active(&KeyAction::LookDown) { self.rotation.x -= rotation_speed }

                if self.is_active(&KeyAction::MouseLook) {
                    self.rotation.y -= self.cursor_position_delta.x * self.settings.mouse_acceleration.y * Self::MOUSE_ACCELERATION_FACTOR;
                    self.rotation.x += self.cursor_position_delta.y * self.settings.mouse_acceleration.x * Self::MOUSE_ACCELERATION_FACTOR;
                    self.cursor_position_delta.fill(0.0);
                }

                if self.rotation.dot(&self.rotation) > f32::EPSILON {
                    self.rotation.scale_mut(tick.delta);
                    movable.rotate(&self.rotation);
                }

                if self.is_active(&KeyAction::MoveForward) { self.translation += movable.forward() }
                if self.is_active(&KeyAction::MoveBackward) { self.translation -= movable.forward() }
                if self.is_active(&KeyAction::StrafeLeft) { self.translation -= movable.right() }
                if self.is_active(&KeyAction::StrafeRight) { self.translation += movable.right() }

                if self.translation.dot(&self.translation) > f32::EPSILON {
                    self.translation.normalize_mut();
                    self.translation.scale_mut(translation_speed * tick.delta);
                    movable.translate(&self.translation);
                }
            }
        }
    }

    fn set_active(&mut self, movement: &KeyAction, active: bool) {
        let update_fn: fn(&mut KeyStateFlags, KeyStateFlags) -> () = if active { KeyStateFlags::insert } else { KeyStateFlags::remove };
        match movement {
            KeyAction::MoveForward=> update_fn(&mut self.key_state_flags, KeyStateFlags::MOVE_FORWARD),
            KeyAction::MoveBackward=> update_fn(&mut self.key_state_flags, KeyStateFlags::MOVE_BACKWARD),
            KeyAction::StrafeLeft=> update_fn(&mut self.key_state_flags, KeyStateFlags::STRAFE_LEFT),
            KeyAction::StrafeRight=> update_fn(&mut self.key_state_flags, KeyStateFlags::STRAFE_RIGHT),
            KeyAction::LookLeft=> update_fn(&mut self.key_state_flags, KeyStateFlags::LOOK_LEFT),
            KeyAction::LookRight=> update_fn(&mut self.key_state_flags, KeyStateFlags::LOOK_RIGHT),
            KeyAction::LookUp=> update_fn(&mut self.key_state_flags, KeyStateFlags::LOOK_UP),
            KeyAction::LookDown=> update_fn(&mut self.key_state_flags, KeyStateFlags::LOOK_DOWN),
            KeyAction::FastMovement=> update_fn(&mut self.key_state_flags, KeyStateFlags::FAST_MOVEMENT),
            KeyAction::MouseLook=> update_fn(&mut self.key_state_flags, KeyStateFlags::MOUSE_LOOK),
            KeyAction::ResetMovement=> update_fn(&mut self.key_state_flags, KeyStateFlags::RESET_MOVEMENT),
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
            KeyAction::ResetMovement => self.key_state_flags.toggle(KeyStateFlags::RESET_MOVEMENT),
        };
    }

    pub fn is_active(&self, movement: &KeyAction) -> bool {
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
            KeyAction::ResetMovement => self.key_state_flags.contains(KeyStateFlags::RESET_MOVEMENT),
        }
    }
}

pub enum KeyAction {
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
    ResetMovement,
}

bitflags! {
    struct KeyStateFlags: u64 {
        const NONE           = 0b0000000000000000;
        const MOVE_FORWARD   = 0b0000000000000001;
        const MOVE_BACKWARD  = 0b0000000000000010;
        const STRAFE_LEFT    = 0b0000000000000100;
        const STRAFE_RIGHT   = 0b0000000000001000;
        const LOOK_LEFT      = 0b0000000000010000;
        const LOOK_RIGHT     = 0b0000000000100000;
        const LOOK_UP        = 0b0000000001000000;
        const LOOK_DOWN      = 0b0000000010000000;
        const FAST_MOVEMENT  = 0b0000000100000000;
        const MOUSE_LOOK     = 0b0000001000000000;
        const RESET_MOVEMENT = 0b0000010000000000;
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
