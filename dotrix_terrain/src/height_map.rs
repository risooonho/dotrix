use rand::Rng;
use std::f32::consts::PI;
use dotrix_core::assets::Texture;

pub struct HeightMap<T> {
    size_x: usize,
    size_z: usize,
    heights: Vec<Vec<T>>,
}

impl<T> HeightMap<T>
where T: From<u8> + std::ops::Mul<Output = T> + Clone + Copy
{

    pub fn new(size_x: usize, size_z: usize) -> Self {
        let len = size_x * size_z;
        let heights = vec![vec![0.into(); size_z]; size_x];
        Self {
            size_x,
            size_z,
            heights,
        }
    }

    pub fn from_texture(texture: &Texture, y_scale: T) -> Self {
        let mut heightmap = Self::new(texture.width as usize, texture.height as usize);

        let bytes_per_pixel = texture.data.len() / heightmap.size_x / heightmap.size_z;
        for x in 0..heightmap.size_x {
            let i = x * heightmap.size_z;
            for z in 0..heightmap.size_z {
                let offset = bytes_per_pixel * (i + z);
                let mut value = 0;
                for b in 0..2 {
                    value |= texture.data[offset + b] << (8 * b);
                }
                heightmap.heights[x][z] = y_scale * value.into();
            }
        }
        heightmap
    }

    pub fn pick<C>(&self, x: C, z: C) -> T
    where Self: Picker<T, C>
    {
        self.pick_height(x, z)
    }

    /*
    pub fn add_hill(&mut self, hill_radius: f32) {
        let mut rng = rand::thread_rng();

        let angle = rng.gen_range(0.0..2.0 * PI);
        let half_size = (self.size / 2) as f32;
        let distance = rng.gen_range((hill_radius / 2.0)..(half_size - hill_radius));
        let x = half_size + angle.cos() * distance;
        let z = half_size + angle.sin() * distance;
        let hill_radius_square = hill_radius * hill_radius;

        let mut x_min = (x - hill_radius - 1.0) as usize;
        let mut x_max = (x + hill_radius + 1.0) as usize;
        let mut z_min = (z - hill_radius - 1.0) as usize;
        let mut z_max = (z + hill_radius + 1.0) as usize;

        if x_max >= self.size {
            x_max = self.size - 1;
        }

        if z_max >= self.size {
            z_max = self.size - 1;
        }

        if x_min > x_max {
            x_min = 0;
        }

        if z_min > z_max {
            z_min = 0;
        }

        for xi in x_min..x_max {
            for zi in z_min..z_max {
                let dx = x - xi as f32;
                let dz = z - zi as f32;
                let height = hill_radius_square - (dx * dx + dz * dz);
                if height > 0.0 {
                    let value = self.heights[zi * self.size + xi];
                    if height > value {
                        self.heights[zi * self.size + xi] = height;
                    }
                }
            }
        }
    }

    pub fn normalize(&mut self) {
        let mut max = 0.0;

        for x in 0..self.size_x {
            for z in 0..self.size_z {
                let height = self.pick(x, z);
                if height > max {
                    max = height;
                }
            }
        }

        if max > 0.0 {
            for x in 0..self.size_x {
                for z in 0..self.size_z {
                    let index = z * self.size_z + x;
                    let height = self.heights[index];
                    self.heights[index] = 50.0 * height / max;
                }
            }
        }
    }
    */
}

pub trait Picker<T, C> {
    fn pick_height(&self, x: C, z: C) -> T;
}

impl<T> Picker<T, usize> for HeightMap<T>
where T: Clone + Copy
{
    fn pick_height(&self, x: usize, z: usize) -> T {
        self.heights[x][z]
    }
}

impl<T> Picker<T, f32> for HeightMap<T>
where T: Clone + Copy
{
    fn pick_height(&self, x: f32, z: f32) -> T
    where Self: Picker<T, usize>
    {
        self.pick_height(z.floor() as usize, x.floor() as usize)
    }
}
