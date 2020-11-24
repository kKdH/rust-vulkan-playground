use cgmath::{Matrix4, Point3, Vector3, Rad};

pub struct Camera {
    pub projection: Matrix4<f32>,
    pub view: Matrix4<f32>,
}

#[derive(Default)]
pub struct  CameraBuilder {
    position: Option<Point3<f32>>,
    center: Option<Point3<f32>>,
    up: Option<Vector3<f32>>,
    near: Option<f32>,
    far: Option<f32>,
    aspect: Option<f32>,
    fov: Option<f32>
}

impl CameraBuilder {

    pub fn move_to(&mut self, point: Point3<f32>) -> &mut CameraBuilder {
        self.position = Some(point);
        self
    }

    pub fn look_at(&mut self, point: Point3<f32>) -> &mut CameraBuilder {
        self.center = Some(point);
        self
    }

    pub fn with_field_of_view(&mut self, value: f32) -> &mut CameraBuilder {
        self.fov = Some(value);
        self
    }

    pub fn with_aspect_ratio_of(&mut self, width: f32, height: f32) -> &mut CameraBuilder {
        self.with_aspect_ratio(width / height)
    }

    pub fn with_aspect_ratio(&mut self, value: f32) -> &mut CameraBuilder {
        self.aspect = Some(value);
        self
    }

    pub fn with_near_plane(&mut self, value: f32) -> &mut CameraBuilder {
        self.near = Some(value);
        self
    }

    pub fn with_far_plane(&mut self, value: f32) -> &mut CameraBuilder {
        self.far = Some(value);
        self
    }

    pub fn build(&self) -> Camera {
        Camera {
            projection: cgmath::perspective(
                Rad(self.fov.expect("No field of view has been specified!")),
                self.aspect.expect("No aspect ration has been specified!"),
                self.near.expect("No near plane has been specified!"),
                self.far.expect("No far plane has been specified!"),
            ),
            view: Matrix4::look_at(
                self.position.unwrap_or(Point3::new(0.0, 0.0, 0.0)),
                self.center.unwrap_or(Point3::new(0.0, 0.0, 0.0)),
                self.up.unwrap_or(Vector3::new(0.0, -1.0, 0.0))
            ),
        }
    }
}

pub fn builder() -> CameraBuilder {
    CameraBuilder::default()
}

#[cfg(test)]
mod test {
    use super::*;
    use cgmath::{Vector3, Rad};
    use hamcrest2::prelude::*;

    #[test]
    fn test_build() {
        let camera = super::builder()
            .move_to(Point3::new(1.0, 1.0, 1.0))
            .look_at(Point3::new(0.0, 0.0, 0.0))
            .with_field_of_view(std::f32::consts::FRAC_PI_2)
            .with_aspect_ratio(1.0)
            .with_near_plane(1.0)
            .with_far_plane(2.0)
            .build();

        assert_that!(camera.view, equal_to(Matrix4::look_at(
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0),
        )));

        assert_that!(camera.projection, equal_to(cgmath::perspective(
            Rad(std::f32::consts::FRAC_PI_2),
            1.0,
            1.0,
            2.0,
        )));
    }
}
