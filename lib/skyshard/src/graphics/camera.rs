use nalgebra::{Matrix4, Vector3};

pub struct Camera {
    projection: Matrix4<f32>,
    view: Matrix4<f32>,
    translation: Vector3<f32>,
    rotation: Vector3<f32>,
    forward: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3::<f32>,
    matrix: Matrix4<f32>,
}

impl Camera {

    pub fn new(projection: Projection) -> Self {

        Self {
            projection: projection.matrix(),
            view: Matrix4::<f32>::identity(),
            translation: Vector3::new(0.0, 0.0, -5.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            forward: Vector3::z(),
            right: Vector3::x(),
            up: Vector3::new(0.0, -1.0, 0.0),
            matrix: Matrix4::<f32>::identity(),
        }
    }

    pub fn view_direction(&mut self, translation: &Vector3<f32>, direction: &Vector3<f32>, up: &Vector3<f32>) {
        let w: Vector3<f32> = direction.normalize();
        let u: Vector3<f32> = w.cross(&up).normalize();
        let v: Vector3<f32> = w.cross(&u);

        self.view[(0, 0)] = u.x;
        self.view[(1, 0)] = u.y;
        self.view[(2, 0)] = u.z;
        self.view[(0, 1)] = v.x;
        self.view[(1, 1)] = v.y;
        self.view[(2, 1)] = v.z;
        self.view[(0, 2)] = w.x;
        self.view[(1, 2)] = w.y;
        self.view[(2, 2)] = w.z;
        self.view[(3, 0)] = -u.dot(&translation);
        self.view[(3, 1)] = -v.dot(&translation);
        self.view[(3, 2)] = -w.dot(&translation);

        self.update();
    }

    pub fn view_target(&mut self, translation: &Vector3<f32>, target: &Vector3<f32>, up: &Vector3<f32>) {
        self.view_direction(translation, &(target - translation), up);
    }

    pub fn view_yxz(&mut self, translation: &Vector3<f32>, rotation: &Vector3<f32>) {

        let c1: f32 = rotation.y.cos();
        let s1: f32 = rotation.y.sin();
        let c2: f32 = rotation.x.cos();
        let s2: f32 = rotation.x.sin();
        let c3: f32 = rotation.z.cos();
        let s3: f32 = rotation.z.sin();

        let u: Vector3<f32> = Vector3::<f32>::new(c1 * c3 + s1 * s2 * s3, c2 * s3, c1 * s2 * s3 - c3 * s1);
        let v: Vector3<f32> = Vector3::<f32>::new(c3 * s1 * s2 - c1 * s3, c2 * c3, c1 * c3 * s2 + s1 * s3);
        let w: Vector3<f32> = Vector3::<f32>::new(c2 * s1, -s2, c1 * c2);

        self.view[(0, 0)] = u.x;
        self.view[(1, 0)] = u.y;
        self.view[(2, 0)] = u.z;
        self.view[(0, 1)] = v.x;
        self.view[(1, 1)] = v.y;
        self.view[(2, 1)] = v.z;
        self.view[(0, 2)] = w.x;
        self.view[(1, 2)] = w.y;
        self.view[(2, 2)] = w.z;
        self.view[(3, 0)] = -u.dot(&translation);
        self.view[(3, 1)] = -v.dot(&translation);
        self.view[(3, 2)] = -w.dot(&translation);

        self.update();
    }

    fn update(&mut self) {

        self.forward.x = self.rotation.y.sin();
        self.forward.z = self.rotation.y.cos();
        self.right.x = self.forward.z;
        self.right.z = -self.forward.x;

        self.matrix.fill_with_identity();
        self.matrix *= self.view;
        self.matrix *= self.projection;
    }

    pub fn as_matrix(&self) -> &Matrix4<f32> {
        &self.matrix
    }
}


#[derive(Debug, Copy, Clone)]
pub enum Projection {
    OrthographicProjection {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    },
    PerspectiveProjection {
        fovy: f32,
        aspect: f32,
        near: f32,
        far: f32,
    },
}

impl Projection {

    pub fn matrix(self) -> Matrix4::<f32> {
        match self {
            Projection::OrthographicProjection { left, right, top, bottom, near, far } => {
                Self::orthographic_projection(left, right, top, bottom, near, far)
            }
            Projection::PerspectiveProjection { fovy, aspect, near, far } => {
                Self::perspective_projection(fovy, aspect, near, far)
            }
        }
    }

    #[inline(always)]
    fn orthographic_projection(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Matrix4::<f32> {
        let mut result = Matrix4::<f32>::identity();
        result[(0, 0)] = 2.0 / (right - left);
        result[(1, 1)] = 2.0 / (bottom - top);
        result[(2, 2)] = 1.0 / (far - near);
        result[(3, 0)] = -(right + left) / (right - left); // row / column maybe interchanged?
        result[(3, 1)] = -(bottom + top) / (bottom - top); // row / column maybe interchanged?
        result[(3, 2)] = -near / (far - near);             // row / column maybe interchanged?
        result
    }

    #[inline(always)]
    fn perspective_projection(fovy: f32, aspect: f32, near: f32, far: f32) -> Matrix4::<f32> {
        debug_assert!((aspect - f32::EPSILON).abs() > 0.0);
        let mut result = Matrix4::<f32>::zeros();
        let tan_half_fovy = (fovy / 2.0).tan();
        result[(0, 0)] = 1.0 / (aspect * tan_half_fovy);
        result[(1, 1)] = 1.0 / (tan_half_fovy);
        result[(2, 2)] = far / (far - near);
        result[(2, 3)] = 1.0;
        result[(3, 2)] = -(far * near) / (far - near);
        result
    }
}
