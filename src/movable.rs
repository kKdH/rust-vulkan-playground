use nalgebra::Vector3;

pub struct Movable {
    rotation: Vector3<f32>,
    translation: Vector3<f32>,
    forward: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
}

impl Movable {

    pub fn new() -> Self {
        Movable {
            translation: Vector3::zeros(),
            rotation: Vector3::zeros(),
            forward: Vector3::z(),
            right: Vector3::x(),
            up: -1f32 * Vector3::y(),
        }
    }

    pub fn rotate(&mut self, value: &Vector3<f32>) {
        self.rotation += value;
        self.update();
    }

    pub fn rotation(&self) -> &Vector3<f32> {
        &self.rotation
    }

    pub fn set_rotation(&mut self, value: &Vector3<f32>) {
        self.rotation.copy_from(value);
        self.update();
    }

    pub fn translate(&mut self, value: &Vector3<f32>) {
        self.translation += value;
    }

    pub fn translation(&self) -> &Vector3<f32> {
        &self.translation
    }

    pub fn set_translation(&mut self, value: &Vector3<f32>) {
        self.translation.copy_from(value);
    }

    pub fn forward(&self) -> &Vector3<f32> {
        &self.forward
    }

    pub fn right(&self) -> &Vector3<f32> {
        &self.right
    }

    pub fn up(&self) -> &Vector3<f32> {
        &self.up
    }

    fn update(&mut self) {
        self.forward.x = self.rotation.y.sin();
        self.forward.z = self.rotation.y.cos();
        self.right.x = self.forward.z;
        self.right.z = -self.forward.x;
    }
}
