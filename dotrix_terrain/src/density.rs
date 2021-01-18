use std::collections::HashMap;

use rayon::prelude::*;

use dotrix_math::Vec3i;

struct Value {
    y: usize,
    value: f32,
}

pub struct Density {
    size_x: usize,
    size_y: usize,
    size_z: usize,
    values: Option<Vec<Vec<Vec<f32>>>>,
    pub zero_at: Vec3i,
}

impl Density {
    pub fn new(size_x: usize, size_y: usize, size_z: usize, zero_at: Vec3i) -> Self {
        Self {
            size_x,
            size_y,
            size_z,
            values: None,
            zero_at,
        }
    }

    pub fn set<F>(&mut self, density: F)
    where F: Fn(usize, usize, usize) -> f32 + Send + Sync
    {
        let values = (0..self.size_x).into_par_iter().map(|x| {
            (0..self.size_y).into_par_iter().map(|y| {
                (0..self.size_z).into_par_iter().map(|z| {
                    density(x, y, z)
                }).collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>();
        self.values = Some(values);
    }

    pub fn value(&self, x: i32, y: i32, z: i32) -> f32 {
        if let Some(values) = self.values.as_ref() {
            let size_x = self.size_x as i32;
            let size_y = self.size_y as i32;
            let size_z = self.size_z as i32;
            let x = self.zero_at.x + x;
            let y = self.zero_at.y + y;
            let z = self.zero_at.z + z;
            if x > 0 && y > 0 && z > 0 && x < size_x && y < size_y && z < size_z {
                let res = values[x as usize][y as usize][z as usize];
                if res > 0.0 {
                    println!("Density @ {}, {}, {} -> {}", x, y, z, res);
                }
                return res;
            }
        }
        if y == 0 { 0.0 } else { -y as f32 }
    }
}
