pub struct Lod(pub usize);

impl Lod {
    pub fn scale(&self) -> i32 {
        (2_i32).pow(self.0 as u32)
    }
}
