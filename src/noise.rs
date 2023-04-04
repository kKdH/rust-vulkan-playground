use std::f32::consts::PI;

use itertools::Itertools;
use nalgebra::Vector3;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use sha2::{Digest, Sha256};


type InterpolationFunction = fn(f32, f32, f32) -> f32;

#[derive(Debug, Copy, Clone)]
pub enum Interpolation {
    Midpoint,
    Linear,
    Cosine,
}

impl Interpolation {

    pub fn function(self) -> InterpolationFunction {
        match self {
            Interpolation::Midpoint => Self::midpoint,
            Interpolation::Linear => Self::linear,
            Interpolation::Cosine => Self::cosine,
        }
    }

    #[inline(always)]
    fn midpoint(low: f32, high: f32, _: f32) -> f32 {
        (low + high) * 0.5f32
    }

    #[inline(always)]
    fn linear(low: f32, high: f32, t: f32) -> f32 {
        low * (1f32 - t) + high * t
    }

    #[inline(always)]
    fn cosine(low: f32, high: f32, t: f32) -> f32 {
        debug_assert!(t >= 0f32 && t <= 1f32, "Expected 0 ≤ t ≤ 1 for cosine-interpolation, but was: {t}");
        Self::linear(low, high, (1f32 - (t * PI).cos() * 0.5f32))
    }
}

struct Noise {
    gradients: Vec<Vector3<f32>>,
    gradients_modulo_mask: i32,
    permutation_table: Vec<usize>,
    interpolate: InterpolationFunction,
}


impl Noise {

    pub fn new(seed: &str, size: usize, interpolation: Interpolation) -> Self {

        debug_assert!(size > 0, "size must be greater than 0");
        debug_assert!((size & (size - 1)) == 0,  "size has to be a power of 2, but was {}", size);

        let seed: [u8; 32] = Sha256::digest(seed).into();
        let mut rng = StdRng::from_seed(seed);

        Self {
            gradients: Clone::clone(&rng)
                .sample_iter(rand::distributions::Uniform::new_inclusive(-1.0, 1.0))
                .chunks(3)
                .into_iter()
                .map(|chunk| chunk
                    .enumerate()
                    .fold(Vector3::zeros(), |mut gradient: Vector3<f32>, (index, value)| {
                        gradient.as_mut_slice()[index] = value;
                        gradient
                    }))
                .map(|mut gradient| {
                    gradient.normalize_mut();
                    gradient
                })
                .take(size)
                .collect(),
            gradients_modulo_mask: (size - 1) as i32,
            permutation_table: rng
                .sample_iter(rand::distributions::Uniform::new(0usize, size))
                .take(2 * size)
                .collect(),
            interpolate: interpolation.function(),
        }
    }

    pub fn evaluate(&self, position: Vector3<f32>) -> f32 {

        let xi0: i32 = self.to_index(position.x.floor() as i32);
        let yi0: i32 = self.to_index(position.y.floor() as i32);
        let zi0: i32 = self.to_index(position.z.floor() as i32);

        let xi1: i32 = self.to_index(xi0 + 1);
        let yi1: i32 = self.to_index(yi0 + 1);
        let zi1: i32 = self.to_index(zi0 + 1);

        let tx = position.x - position.x.floor();
        let ty = position.y - position.y.floor();
        let tz = position.z - position.z.floor();

        let u = Self::quintic(tx);
        let v = Self::quintic(ty);
        let w = Self::quintic(tz);

        let c000: &Vector3<f32> = self.gradient_at(xi0, yi0, zi0);
        let c100: &Vector3<f32> = self.gradient_at(xi1, yi0, zi0);
        let c010: &Vector3<f32> = self.gradient_at(xi0, yi1, zi0);
        let c110: &Vector3<f32> = self.gradient_at(xi1, yi1, zi0);
        let c001: &Vector3<f32> = self.gradient_at(xi0, yi0, zi1);
        let c101: &Vector3<f32> = self.gradient_at(xi1, yi0, zi1);
        let c011: &Vector3<f32> = self.gradient_at(xi0, yi1, zi1);
        let c111: &Vector3<f32> = self.gradient_at(xi1, yi1, zi1);

        let x0 = tx;
        let x1 = tx - 1.0;
        let y0 = ty;
        let y1 = ty - 1.0;
        let z0 = tz;
        let z1 = tz - 1.0;

        let p000 = Vector3::<f32>::new(x0, y0, z0);
        let p100 = Vector3::<f32>::new(x1, y0, z0);
        let p010 = Vector3::<f32>::new(x0, y1, z0);
        let p110 = Vector3::<f32>::new(x1, y1, z0);
        let p001 = Vector3::<f32>::new(x0, y0, z1);
        let p101 = Vector3::<f32>::new(x1, y0, z1);
        let p011 = Vector3::<f32>::new(x0, y1, z1);
        let p111 = Vector3::<f32>::new(x1, y1, z1);

        let a = (self.interpolate)(c000.dot(&p000), c100.dot(&p100), u);
        let b = (self.interpolate)(c010.dot(&p010), c110.dot(&p110), u);
        let c = (self.interpolate)(c001.dot(&p001), c101.dot(&p101), u);
        let d = (self.interpolate)(c011.dot(&p011), c111.dot(&p111), u);

        let e = (self.interpolate)(a, b, v);
        let f = (self.interpolate)(c, d, v);

        (self.interpolate)(e, f, w)
    }

    #[inline(always)]
    fn to_index(&self, value: i32) -> i32 {
        value & self.gradients_modulo_mask
    }

    #[inline(always)]
    fn gradient_at(&self, x: i32, y: i32, z: i32) -> &Vector3<f32> {
        let index = self.permutation_table[self.permutation_table[self.permutation_table[x as usize] + y as usize] + z as usize];
        #[cfg(debug_assertions)]
        return self.gradients.get(index)
            .expect(&format!("index should be in the range [0,{}) to get a gradient, but was {}", self.gradients.len(), index));
        #[cfg(not(debug_assertions))]
        return unsafe { self.gradients.get_unchecked(index) };
    }

    #[inline(always)]
    fn quintic(t: f32) -> f32 {
        t * t * t * (t * (t * 6f32 - 15f32) + 10f32)
    }

    #[inline(always)]
    fn quintic_derivative(t: f32) -> f32 {
        30f32 * t * t * (t * (t - 2f32) + 1f32)
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;

    use nalgebra::Vector3;
    use png::ColorType;

    use crate::noise::{Interpolation, Noise};

    #[test]
    fn test_noise() {

        let noise = Noise::new("Elmar", 256, Interpolation::Linear);

        let image_width: usize = 256;
        let image_height: usize = 256;
        let mut noise_map = vec![0f32; image_width * image_height];
        let mut frequency = 1.00f32;
        let mut amplitude = 1.0f32;
        let lacunarity = 0.25;
        let gain = 1.8;
        let layers = 5;

        let mut min_value = f32::MAX;
        let mut max_value = f32::MIN;

        for j in 0..image_height {
            for i in 0..image_width {
                frequency = 0.5;
                amplitude = 0.5;
                for l in 0..layers {
                    let value = noise.evaluate(Vector3::new(i as f32, j as f32, 0.0) * frequency) * amplitude;
                    if value > max_value { max_value = value }
                    if value < min_value { min_value = value }
                    noise_map[j * image_width + i] += value;
                    frequency *= lacunarity;
                    amplitude *= gain;
                }
            }
        }

        println!("min={min_value}, max={max_value}");

        let path = Path::new(r"tmp/image.png");
        let file = File::create(path).unwrap();
        let ref mut writer = BufWriter::new(file);
        let mut encoder = ::png::Encoder::new(writer, 1 * image_width as u32, 1 * image_height as u32);
        encoder.set_color(ColorType::Grayscale);
        let mut png_writer = encoder.write_header().unwrap();

        let data: Vec<u8> = noise_map
            .iter()
            .map(|value| (value - min_value) / (max_value - min_value))
            .map(|value| (255f32 - value * 255f32) as u8)
            .collect();

        png_writer.write_image_data(data.as_slice()).unwrap();
    }
}
