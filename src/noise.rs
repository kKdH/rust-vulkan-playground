use std::f32::consts::PI;

use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand::distributions::Standard;
use rand::rngs::StdRng;
use sha2::{Digest, Sha256};

type InterpolationFunction = fn(f32, f32, f32) -> f32;

#[derive(Copy, Clone)]
pub enum Interpolation {
    Linear,
    Cosine,
}

impl Interpolation {

    pub fn function(self) -> InterpolationFunction {
        match self {
            Interpolation::Linear => Self::linear,
            Interpolation::Cosine => Self::cosine,
        }
    }

    #[inline(always)]
    fn linear(low: f32, high: f32, t: f32) -> f32 {
        low * (1f32 - t) + high * t
    }

    #[inline(always)]
    fn cosine(low: f32, high: f32, t: f32) -> f32 {
        debug_assert!(t >= 0f32 && t <= 1f32);
        Self::linear(low, high, (1f32 - (t * PI).cos() * 0.5f32))
    }
}

struct Noise {
    random_table: Vec<f32>,
    random_table_mask: i32,
    interpolate: InterpolationFunction,
}

impl Noise {

    pub fn new(seed: &str, size: usize, interpolation: Interpolation) -> Self {

        let seed: [u8; 32] = Sha256::digest(seed).into();
        let mut rng = StdRng::from_seed(seed);

        Self {
            random_table: rng.sample_iter(Standard).take(size * size).collect(),
            random_table_mask: (size - 1) as i32,
            interpolate: interpolation.function(),
        }
    }

    pub fn evaluate(&self, position: Vector2<f32>) -> f32 {

        let xi: i32 = position.x.floor() as i32;
        let yi: i32 = position.y.floor() as i32;

        let tx = position.x - xi as f32;
        let ty = position.y - yi as f32;

        let rx0 = xi & self.random_table_mask;
        let rx1 = (rx0 + 1) & self.random_table_mask;
        let ry0 = yi & self.random_table_mask;
        let ry1 = (ry0 + 1) & self.random_table_mask;

        let c00 = self.random_table[(ry0 * self.random_table_mask + rx0) as usize];
        let c01 = self.random_table[(ry0 * self.random_table_mask + rx1) as usize];
        let c10 = self.random_table[(ry1 * self.random_table_mask + rx0) as usize];
        let c11 = self.random_table[(ry1 * self.random_table_mask + rx1) as usize];

        let sx = Self::smooth(tx);
        let sy = Self::smooth(ty);

        let nx0 = (self.interpolate)(c00, c10, sx);
        let nx1 = (self.interpolate)(c01, c11, sx);

        (self.interpolate)(nx0, nx1, sy)
    }

    #[inline(always)]
    fn smooth(t: f32) -> f32 {
        t * t * (3f32 - 2f32 * t)
    }
}

struct Generation {
    frequency: f32,
    lacunarity: f32,
    amplitude: f32,
    gain: f32,
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;

    use nalgebra::Vector2;
    use png::ColorType;

    use crate::noise::{Interpolation, Noise};

    #[test]
    fn test_noise() {

        let noise = Noise::new("Mina", 256, Interpolation::Linear);


        let image_width: usize = 128;
        let image_height: usize = 128;
        let mut noise_map = vec![0f32; image_width * image_height];
        let frequency = 1.0f32;
        let amplitude = 1.0f32;

        for j in 0..image_height {
            for i in 0..image_width {
                noise_map[j * image_width + i] = noise.evaluate(Vector2::new(i as f32, j as f32) * frequency) * amplitude;
            }
        }

        let path = Path::new(r"tmp/image.png");
        let file = File::create(path).unwrap();
        let ref mut writer = BufWriter::new(file);
        let mut encoder = ::png::Encoder::new(writer, image_width as u32, image_height as u32);
        encoder.set_color(ColorType::Rgb);
        let mut png_writer = encoder.write_header().unwrap();

        let data = noise_map
            .iter()
            .map(|value| (value * 255f32) as u8)
            .fold(Vec::<u8>::new(), | mut result, value| {
                result.push(value);
                result.push(value);
                result.push(value);
                result
        });

        png_writer.write_image_data(data.as_slice()).unwrap();
    }

    #[test]
    fn test_noise_for_negativ_position() {

        let noise = Noise::new("Hello World", 64, Interpolation::Linear);

        assert_eq!(noise.evaluate(Vector2::new(-1.0, 0.0)), 0.17040652);
        assert_eq!(noise.evaluate(Vector2::new(0.0, -1.0)), 0.008818567);
        assert_eq!(noise.evaluate(Vector2::new(-1.0, -1.0)), 0.69496435);
    }
}
