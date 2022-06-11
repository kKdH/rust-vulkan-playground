use nalgebra::{Matrix4, UnitQuaternion, Vector3};

/// Camera
///
/// Projection:
/// World Coordinate system
///
/// +y
/// |  +z
/// | /
/// |/___+x
///
/// NDC system
///
///  -z
/// /
/// |¯¯¯+x
/// |
/// +y
///
pub struct Camera {
    projection: Matrix4<f32>,
    view: Matrix4<f32>,
    position: Vector3<f32>,
    direction: Vector3<f32>,
    pitch: UnitQuaternion<f32>,
    roll: UnitQuaternion<f32>,
    yaw: UnitQuaternion<f32>,
    matrix: Matrix4<f32>,
}

impl Camera {

    pub fn new(aspect: f32, fov: f32, z_near: f32, z_far: f32) -> Self {

        let mut projection = Matrix4::<f32>::zeros();
        let tan_half_fovy = (fov / 2.0).tan();

        projection[(0, 0)] = 1.0 / (aspect * tan_half_fovy);
        projection[(1, 1)] = -1.0 / tan_half_fovy;
        projection[(2, 2)] = z_far / (z_near - z_far);
        projection[(2, 3)] = -(z_far * z_near) / (z_far - z_near);
        projection[(3, 2)] = -1.0;

        Self {
            projection,
            view: Matrix4::<f32>::identity(),
            position: Vector3::zeros(),
            direction: Vector3::zeros(),
            pitch: UnitQuaternion::identity(),
            roll: UnitQuaternion::identity(),
            yaw: UnitQuaternion::identity(),
            matrix: projection,
        }
    }

    pub fn roll(&mut self, angle: f32) {
        self.roll = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), angle.to_radians());
    }

    pub fn pitch(&mut self, angle: f32) {
        self.pitch = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), angle.to_radians());
    }

    pub fn yaw(&mut self, angle: f32) {
        self.yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), angle.to_radians());
    }

    pub fn eye(&mut self, x: f32, y: f32, z: f32) {
        self.position[0] = x;
        self.position[1] = y;
        self.position[2] = z;
    }

    pub fn forward(&mut self) {
        self.position += self.direction
    }

    pub fn backward(&mut self) {
        self.position -= self.direction
    }

    pub fn strafe_left(&mut self) {
        self.position += self.direction.cross(&Vector3::y_axis())
    }

    pub fn strafe_right(&mut self) {
        self.position -= self.direction.cross(&Vector3::y_axis())
    }

    pub fn update(&mut self) {
        let orientation = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), std::f32::consts::PI) * self.roll * self.pitch * self.yaw;
        // self.direction = (orientation * Vector3::new(0f32, 0f32, 1f32));
        self.direction = orientation.transform_vector(&Vector3::z_axis());
        self.direction.scale_mut(0.2);

        let mut translation = Matrix4::<f32>::identity();
        translation[(0, 3)] = self.position[0];
        translation[(1, 3)] = self.position[1] * -1.0; // due to y-down?
        translation[(2, 3)] = self.position[2];
        self.view = orientation.to_rotation_matrix().to_homogeneous() * translation;
        self.matrix = self.projection * self.view;
    }

    pub fn reset(&mut self) {
        self.position = Vector3::new(0f32, 0f32, 3f32);
        self.roll(0.0);
        self.pitch(0.0);
        self.yaw(0.0);
    }

    #[inline]
    pub fn as_matrix(&self) -> &Matrix4<f32> {
        &self.matrix
    }
}
