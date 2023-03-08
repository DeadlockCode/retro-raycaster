pub struct Texture {
    data: Vec<u32>,
    width: u32,
    height: u32,
}

impl Texture {
    pub fn new(path: &str) -> Self {
        let decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = buf[..info.buffer_size()].to_vec().chunks_exact(3).map(|chunk| {
            unsafe {
                std::mem::transmute::<[u8; 4], u32>([0xff, chunk[2], chunk[1], chunk[0]])
            }
        }).collect::<Vec<_>>();

        Self {
            data,
            width: info.width,
            height: info.height,
        }
    }

    /// Wraping and out of bounds errors are possible if u and v aren't in the range of [0, 1)
    pub fn get(&self, u: f32, v: f32) -> u32 { 
        let x = (u * self.width as f32).floor() as usize;
        let y = (v * self.height as f32).floor() as usize;
        self.data[x + y * self.width as usize]
    }
}