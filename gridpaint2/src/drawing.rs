#[derive(Copy, Clone)]
pub struct RGBA(pub u8, pub u8, pub u8, pub u8);

const TRANS1: RGBA = RGBA(0x66, 0x66, 0x66, 255);
const TRANS2: RGBA = RGBA(0x99, 0x99, 0x99, 255);

impl From<RGBA> for u32 {
    fn from(value: RGBA) -> Self {
        let ret: u32 = ((value.0 as u32) << 24)
            | ((value.1 as u32) << 16)
            | ((value.2 as u32) << 8)
            | (value.3 as u32);
        ret.to_be()
    }
}

fn idx_1dto2d(x: usize, y: usize, width: usize) -> usize {
    x + y * width
}

pub fn draw_bg(image: &mut [u32], w: u32, h: u32, cw: u32, ch: u32, xmax: u32) {
    for y in 0..(h * 2) {
        for x in 0..(w * 2) {
            let pix = if (x + (y & 1)) & 1 == 1 {
                TRANS1
            } else {
                TRANS2
            };
            fill_rect(image, pix, x, y, cw / 2, ch / 2, xmax * 2);
        }
    }
}

// pub fn draw_grid(image: &mut [u32], height: u32, width: u32) {}

pub fn fill_rect(image: &mut [u32], pixel: RGBA, x: u32, y: u32, cw: u32, ch: u32, xmax: u32) {
    let base_x = x * cw;
    let base_y = y * ch;

    // Each pixel needs to occupy a (w * h) square around its position.
    for y in base_y..(base_y + ch) {
        for x in base_x..(base_x + cw) {
            image[idx_1dto2d(x as usize, y as usize, (cw * xmax) as usize)] = pixel.into();
        }
    }
}
