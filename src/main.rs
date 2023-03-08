use std::f32::consts::PI;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::VirtualKeyCode;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod math;
use math::Vec2;

mod texture;
use texture::Texture;

const PIXEL: usize = 4;

const WIDTH: usize = 1280 / PIXEL;
const HEIGHT: usize = 720 / PIXEL;

const FOV: f32 = PI * 0.5;

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new((WIDTH * PIXEL) as f64, (HEIGHT * PIXEL) as f64);
        WindowBuilder::new()
            .with_resizable(false)
            .with_title("Hello Pixels")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let surface_texture = SurfaceTexture::new((WIDTH * PIXEL) as u32, (HEIGHT * PIXEL) as u32, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };
    let mut world = World::new();

    event_loop.run(move |event, _, control_flow| {
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() || input.destroyed() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Update internal state
            world.update(&input);

            // Draw internal state
            let frame = pixels.get_frame_mut();
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    set_pixel(x, y, 0x000000ff, frame)
                }
            }
            world.draw(frame);

            // Render pixels
            if let Err(err) = pixels.render() {
                error!("pixels.render() failed: {err}");
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
    });
}

struct World {
    pos: Vec2,
    angle: f32,
    vertices: Vec<Vec2>,
    indices: Vec<(usize, usize)>,
    u: Vec<(f32, f32)>,
    wall_tex: Texture,
}

impl World {
    fn new() -> Self {
        Self {
            pos: Vec2::ZERO,
            angle: 0.0,
            vertices: vec![
                (0.0, 4.0).into(),
                (3.0, 2.0).into(),
                (5.0, -2.0).into(),
                (1.0, -4.0).into(),
                (-3.0, -2.0).into(),
                (-3.0, 0.0).into(),
                (-1.0, 0.0).into(),
                (-1.0, 2.0).into(),
                (-3.0, 2.0).into(),
                (-2.0, 4.0).into(),
            ],
            indices: vec![
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 8),
                (8, 9),
                (9, 0),
            ],
            u: vec![
                (0.0, 3.6),
                (3.6, 8.1),
                (8.1, 12.6),
                (12.6, 17.1),
                (17.1, 19.1),
                (19.1, 21.1),
                (21.1, 23.1),
                (23.1, 25.1),
                (25.1, 27.3),
                (27.3, 29.3),
            ],
            wall_tex: Texture::new("assets\\rock.png"),
        }
    }

    fn update(&mut self, input: &WinitInputHelper) {
        self.angle -= input.mouse_diff().0 * 0.01;

        let w = input.key_held(VirtualKeyCode::W) as u8 as f32;
        let a = input.key_held(VirtualKeyCode::A) as u8 as f32;
        let s = input.key_held(VirtualKeyCode::S) as u8 as f32;
        let d = input.key_held(VirtualKeyCode::D) as u8 as f32;

        let movement = Vec2::new(d - a, w - s).rotate(self.angle);

        self.pos += movement * 0.05;

    }

    fn draw(&self, frame: &mut [u8]) {

        let near = (WIDTH / 2) as f32 / (FOV / 2.0).tan();

        (0..WIDTH).map(|x| {
            let dir = Vec2::new((x as i32 - (WIDTH as i32) / 2) as f32 / near, 1.0);

            (x, dir)
        }).for_each(|(x, dir)| {
            let tu = self.indices.iter().map(|&(i, j)| { 
                (self.vertices[i], self.vertices[j]) 
            }).map(|(p0, p1)| {
                ((p0 - self.pos).rotate(-self.angle), (p1 - self.pos).rotate(-self.angle))
            }).enumerate().filter_map(|(i, (p0, p1))| {
                let diff = p1 - p0;
                let s = dir.cross(diff);

                if s == 0.0 { // Collinear
                    return None;
                }

                let t = p0.cross(diff) / s;
                let u = p0.cross(dir) / s;

                if 0.0 <= u && u <= 1.0 && t > 0.0 { // Intersects if u ∈ [0, 1] & t ∈ (0, ∞]
                    let map_u = self.u[i];
                    return Some((t, u * (map_u.1 - map_u.0) + map_u.0));
                }
                None
            }).min_by(|(t0, _), (t1, _)| { t0.total_cmp(t1) });


            if let Some((t, u)) = tu {
                let height = (2.0 * near / t) as usize;
                let y0 = (HEIGHT - height.min(HEIGHT)) / 2;
                let y1 = (HEIGHT + height.min(HEIGHT)) / 2;

                let v0 = 0.5 - (HEIGHT as f32 / height as f32).min(1.0) * 0.5;

                vertical_line_tex(x, y0, y1, u, 1.0 - v0, v0, &self.wall_tex, frame);
            }
        });
    }
}

fn vertical_line_tex(x: usize, y0: usize, y1: usize, u: f32, v0: f32, v1: f32, tex: &Texture, frame: &mut [u8]) {
    for y in y0..y1 {
        let v = ((y - y0) as f32 / (y1 - y0) as f32) * (v1 - v0) + v0;
        set_pixel(x, y, tex.get(u % 1.0, v % 1.0), frame);
    }
}

fn set_pixel(x: usize, y: usize, color: u32, frame: &mut [u8]) {
    let i = x + (HEIGHT - 1 - y) * WIDTH;
    frame[i * 4 + 0] = ((color >> 24) & 0xff) as u8;
    frame[i * 4 + 1] = ((color >> 16) & 0xff) as u8;
    frame[i * 4 + 2] = ((color >> 8)  & 0xff) as u8;
    frame[i * 4 + 3] = ((color >> 0)  & 0xff) as u8;
}


/* Drawing methods used during genesis for testing
fn draw_rect(x: usize, y: usize, w: usize, h: usize, frame: &mut [u8]) {
    for y in y..y+h {
        for x in x..x+w {
            set_pixel(x, y, 0xffffffff, frame);
        }
    }
}

fn draw_circle(x: usize, y: usize, r: f32, color: u32, fill: bool, frame: &mut [u8]) {
    fn not_filled(x: usize, y: usize, i: usize, j: usize, color: u32, frame: &mut [u8]) {
        set_pixel(x + i, y + j, color, frame);
        set_pixel(x + i, y - j, color, frame);
        set_pixel(x - i, y + j, color, frame);
        set_pixel(x - i, y - j, color, frame);
        set_pixel(x + j, y + i, color, frame);
        set_pixel(x + j, y - i, color, frame);
        set_pixel(x - j, y + i, color, frame);
        set_pixel(x - j, y - i, color, frame);
    }

    fn filled(x: usize, y: usize, i: usize, j: usize, color: u32, frame: &mut [u8]) {
        horizontal_line(x - i, x + i, y + j, color, frame);
        horizontal_line(x - i, x + i, y - j, color, frame);
        horizontal_line(x - j, x + j, y + i, color, frame);
        horizontal_line(x - j, x + j, y - i, color, frame);
    }

    let mut i = 0.0;
    let mut j = r;
    let mut d = 3.0 - (2.0 * r);

    if fill {
        filled(x, y, i as usize, j as usize, color, frame);
    } else {
        not_filled(x, y, i as usize, j as usize, color, frame);
    }

    while i <= j {
        if d <= 0.0 {
            d += (4.0 * i) + 6.0;
        }  
        else  
        {
            d += (4.0 * i) - (4.0 * j) + 10.0;
            j -= 1.0;
        }
        i += 1.0;

        if fill {
            filled(x, y, i as usize, j as usize, color, frame);
        } else {
            not_filled(x, y, i as usize, j as usize, color, frame);
        }
    }
}

fn draw_line(mut x0: usize, mut y0: usize, x1: usize, y1: usize, color: u32, frame: &mut [u8]) {
    let dx = (x1 as f32 - x0 as f32).abs();
    let dy = -(y1 as f32 - y0 as f32).abs();
    let mut error = dx + dy;

    loop {
        set_pixel(x0, y0, 0xffffffff, frame);
        if x0 == x1 && y0 == y1 { break; }
        let e2 = 2.0 * error;
        if e2 >= dy {
            if x0 == x1 { break; }
            error += dy;
            if x0 < x1 {
                x0 += 1;
            }
            else {
                x0 -= 1;
            }
        }
        if e2 <= dx {
            if y0 == y1 { break; }
            error += dx;
            if y0 < y1 {
                y0 += 1;
            }
            else {
                y0 -= 1;
            }
        }
    }
}

fn horizontal_line(x0: usize, x1: usize, y: usize, color: u32, frame: &mut [u8]) {
    for x in x0..x1 {
        set_pixel(x, y, color, frame);
    }
}

fn vertical_line(x: usize, y0: usize, y1: usize, color: u32, frame: &mut [u8]) {
    for y in y0..y1 {
        set_pixel(x, y, color, frame);
    }
}
*/