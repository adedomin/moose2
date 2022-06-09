use crate::moosedb::{Dimensions, Moose, PIX_FMT_HEIGHT, PIX_FMT_WIDTH};
use std::cmp::Ordering::{Equal, Greater, Less};

#[derive(Copy, Clone)]
pub struct RGBA(u8, u8, u8, u8);

impl From<RGBA> for (u8, u8, u8) {
    fn from(r: RGBA) -> Self {
        (r.0, r.1, r.2)
    }
}

pub const EXTENDED_COLORS: [RGBA; 100] = [
    // legacy mIRC colors
    RGBA(0xff, 0xff, 0xff, 0xff), // white
    RGBA(0x00, 0x00, 0x00, 0xff), // black
    RGBA(0x00, 0x00, 0x80, 0xff), // navy
    RGBA(0x00, 0x80, 0x00, 0xff), // green
    RGBA(0xff, 0x00, 0x00, 0xff), // red
    RGBA(0xa5, 0x2a, 0x2a, 0xff), // brown
    RGBA(0x80, 0x00, 0x80, 0xff), // purple
    RGBA(0x80, 0x80, 0x00, 0xff), // olive
    RGBA(0xff, 0xff, 0x00, 0xff), // yellow
    RGBA(0x00, 0xff, 0x00, 0xff), // lime
    RGBA(0x00, 0x80, 0x80, 0xff), // teal
    RGBA(0x00, 0xff, 0xff, 0xff), // cyan
    RGBA(0x00, 0x00, 0xff, 0xff), // blue
    RGBA(0xff, 0x00, 0xff, 0xff), // fuchsia
    RGBA(0x80, 0x80, 0x80, 0xff), // grey
    RGBA(0xd3, 0xd3, 0xd3, 0xff), // lightgrey
    // extended mIRC Colors
    // darkest
    RGBA(0x47, 0x00, 0x00, 0xff), // code 16 0
    RGBA(0x47, 0x21, 0x00, 0xff), // code 17 1
    RGBA(0x47, 0x47, 0x00, 0xff), // code 18 2
    RGBA(0x32, 0x47, 0x00, 0xff), // code 19 3
    RGBA(0x00, 0x47, 0x00, 0xff), // code 20 4
    RGBA(0x00, 0x47, 0x2c, 0xff), // code 21 5
    RGBA(0x00, 0x47, 0x47, 0xff), // code 22 6
    RGBA(0x00, 0x27, 0x47, 0xff), // code 23 7
    RGBA(0x00, 0x00, 0x47, 0xff), // code 24 8
    RGBA(0x2e, 0x00, 0x47, 0xff), // code 25 9
    RGBA(0x47, 0x00, 0x47, 0xff), // code 26 a
    RGBA(0x47, 0x00, 0x2a, 0xff), // code 27 b
    RGBA(0x74, 0x00, 0x00, 0xff), // code 28
    RGBA(0x74, 0x3a, 0x00, 0xff), // code 29
    RGBA(0x74, 0x74, 0x00, 0xff), // code 30
    RGBA(0x51, 0x74, 0x00, 0xff), // code 31
    RGBA(0x00, 0x74, 0x00, 0xff), // code 32
    RGBA(0x00, 0x74, 0x49, 0xff), // code 33
    RGBA(0x00, 0x74, 0x74, 0xff), // code 34
    RGBA(0x00, 0x40, 0x74, 0xff), // code 35
    RGBA(0x00, 0x00, 0x74, 0xff), // code 36
    RGBA(0x4b, 0x00, 0x74, 0xff), // code 37
    RGBA(0x74, 0x00, 0x74, 0xff), // code 38
    RGBA(0x74, 0x00, 0x45, 0xff), // code 39
    RGBA(0xb5, 0x00, 0x00, 0xff), // code 40
    RGBA(0xb5, 0x63, 0x00, 0xff), // code 41
    RGBA(0xb5, 0xb5, 0x00, 0xff), // code 42
    RGBA(0x7d, 0xb5, 0x00, 0xff), // code 43
    RGBA(0x00, 0xb5, 0x00, 0xff), // code 44
    RGBA(0x00, 0xb5, 0x71, 0xff), // code 45
    RGBA(0x00, 0xb5, 0xb5, 0xff), // code 46
    RGBA(0x00, 0x63, 0xb5, 0xff), // code 47
    RGBA(0x00, 0x00, 0xb5, 0xff), // code 48
    RGBA(0x75, 0x00, 0xb5, 0xff), // code 49
    RGBA(0xb5, 0x00, 0xb5, 0xff), // code 50
    RGBA(0xb5, 0x00, 0x6b, 0xff), // code 51 end of column
    RGBA(0xff, 0x00, 0x00, 0xff), // code 52
    RGBA(0xff, 0x8c, 0x00, 0xff), // code 53
    RGBA(0xff, 0xff, 0x00, 0xff), // code 54
    RGBA(0xb2, 0xff, 0x00, 0xff), // code 55
    RGBA(0x00, 0xff, 0x00, 0xff), // code 56
    RGBA(0x00, 0xff, 0xa0, 0xff), // code 57
    RGBA(0x00, 0xff, 0xff, 0xff), // code 58
    RGBA(0x00, 0x8c, 0xff, 0xff), // code 59
    RGBA(0x00, 0x00, 0xff, 0xff), // code 60
    RGBA(0xa5, 0x00, 0xff, 0xff), // code 61
    RGBA(0xff, 0x00, 0xff, 0xff), // code 62
    RGBA(0xff, 0x00, 0x98, 0xff), // code 63
    RGBA(0xff, 0x59, 0x59, 0xff), // code 64
    RGBA(0xff, 0xb4, 0x59, 0xff), // code 65
    RGBA(0xff, 0xff, 0x71, 0xff), // code 66
    RGBA(0xcf, 0xff, 0x60, 0xff), // code 67
    RGBA(0x6f, 0xff, 0x6f, 0xff), // code 68
    RGBA(0x65, 0xff, 0xc9, 0xff), // code 69
    RGBA(0x6d, 0xff, 0xff, 0xff), // code 70
    RGBA(0x59, 0xb4, 0xff, 0xff), // code 71
    RGBA(0x59, 0x59, 0xff, 0xff), // code 72
    RGBA(0xc4, 0x59, 0xff, 0xff), // code 73
    RGBA(0xff, 0x66, 0xff, 0xff), // code 74
    RGBA(0xff, 0x59, 0xbc, 0xff), // code 75
    // lightest
    RGBA(0xff, 0x9c, 0x9c, 0xff), // code 76
    RGBA(0xff, 0xd3, 0x9c, 0xff), // code 77
    RGBA(0xff, 0xff, 0x9c, 0xff), // code 78
    RGBA(0xe2, 0xff, 0x9c, 0xff), // code 79
    RGBA(0x9c, 0xff, 0x9c, 0xff), // code 80
    RGBA(0x9c, 0xff, 0xdb, 0xff), // code 81
    RGBA(0x9c, 0xff, 0xff, 0xff), // code 82
    RGBA(0x9c, 0xd3, 0xff, 0xff), // code 83
    RGBA(0x9c, 0x9c, 0xff, 0xff), // code 84
    RGBA(0xdc, 0x9c, 0xff, 0xff), // code 85
    RGBA(0xff, 0x9c, 0xff, 0xff), // code 86
    RGBA(0xff, 0x94, 0xd3, 0xff), // code 87
    RGBA(0x00, 0x00, 0x00, 0xff), // code 88 - blackest
    RGBA(0x13, 0x13, 0x13, 0xff), // code 89
    RGBA(0x28, 0x28, 0x28, 0xff), // code 90
    RGBA(0x36, 0x36, 0x36, 0xff), // code 91
    RGBA(0x4d, 0x4d, 0x4d, 0xff), // code 92
    RGBA(0x65, 0x65, 0x65, 0xff), // code 93
    RGBA(0x81, 0x81, 0x81, 0xff), // code 94
    RGBA(0x9f, 0x9f, 0x9f, 0xff), // code 95
    RGBA(0xbc, 0xbc, 0xbc, 0xff), // code 96
    RGBA(0xe2, 0xe2, 0xe2, 0xff), // code 97
    RGBA(0xff, 0xff, 0xff, 0xff), // code 98 - whitest
    RGBA(0x00, 0x00, 0x00, 0x00), // transparent (code 99)
];

/// PNG Indexed color palette
const PLTE: [u8; EXTENDED_COLORS.len() * 3] = {
    let mut a = [0x00u8; EXTENDED_COLORS.len() * 3];
    let mut i = 0;
    loop {
        if i == EXTENDED_COLORS.len() {
            break a;
        }
        let j = i * 3;
        a[j] = EXTENDED_COLORS[i].0;
        a[j + 1] = EXTENDED_COLORS[i].1;
        a[j + 2] = EXTENDED_COLORS[i].2;
        i += 1;
    }
};

/// PNG section that defines indexed colors' 8bit alpha channel.
const TRNS: [u8; EXTENDED_COLORS.len()] = {
    let mut a = [0xFFu8; EXTENDED_COLORS.len()];
    // only the last color is transparent
    a[EXTENDED_COLORS.len() - 1] = 0x00u8;
    a
};

pub const TRANSPARENT: u8 = 99u8;

fn pix_char(pixel: u8) -> u8 {
    if pixel == TRANSPARENT {
        b' '
    } else {
        b'@'
    }
}

fn single_pixel_term(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        b"\x1b[0m ".to_vec()
    } else {
        let (r, g, b) = EXTENDED_COLORS[pixel as usize].into();
        format!("\x1b[48;2;{0};{1};{2}m ", r, g, b).into()
    }
}

fn single_pixel(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        vec![b'\x03', b' ']
    } else {
        format!("\x03{0},{0}{1}", pixel, pix_char(pixel) as char).into()
    }
}

fn is_same(row: &&[u8]) -> bool {
    row.iter().all(|&pixel| pixel == TRANSPARENT)
}

fn trim_moose<'m>(image: &'m [u8], dim: &Dimensions) -> Vec<&'m [u8]> {
    let (dim_x, _dim_y, _total) = dim.width_height();
    let partials = image
        .chunks_exact(dim_x)
        .skip_while(is_same)
        .collect::<Vec<&'m [u8]>>()
        .iter()
        .rev()
        .skip_while(|&row| is_same(row))
        .cloned()
        .collect::<Vec<&'m [u8]>>()
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<&'m [u8]>>();

    if let Some((left_trim, right_trim)) = partials
        .iter()
        .map(|row| {
            let left = row
                .iter()
                .take_while(|&&pixel| pixel == TRANSPARENT)
                .count();
            let right = row
                .iter()
                .rev()
                .take_while(|&&pixel| pixel == TRANSPARENT)
                .count();
            (left, right)
        })
        .reduce(|(l1, r1), (l2, r2)| {
            let lret = match l1.cmp(&l2) {
                Less | Equal => l1,
                Greater => l2,
            };
            let rret = match r1.cmp(&r2) {
                Less | Equal => r1,
                Greater => r2,
            };
            (lret, rret)
        })
    {
        partials
            .iter()
            .map(|&row| &row[left_trim..(row.len() - right_trim)])
            .collect::<Vec<&'m [u8]>>()
    } else {
        partials
    }
}

pub enum LineType {
    IrcArt,
    TrueColorTerm,
}

pub fn moose_irc(moose: &Moose) -> Vec<u8> {
    moose_line(moose, LineType::IrcArt)
}

pub fn moose_term(moose: &Moose) -> Vec<u8> {
    moose_line(moose, LineType::TrueColorTerm)
}

pub fn moose_line(moose: &Moose, l: LineType) -> Vec<u8> {
    let mut moose_image = trim_moose(&moose.image, &moose.dimensions);

    let mut ret = moose_image
        .drain(..)
        .flat_map(|row| {
            let mut out_row = vec![];
            let mut last_pix = 100u8;
            if let LineType::IrcArt = l {
                for &pixel in row {
                    if pixel == last_pix {
                        out_row.push(pix_char(pixel))
                    } else {
                        last_pix = pixel;
                        out_row.extend(single_pixel(pixel));
                    }
                }
            } else {
                for &pixel in row {
                    if pixel == last_pix {
                        out_row.push(b' ');
                    } else {
                        last_pix = pixel;
                        out_row.extend(single_pixel_term(pixel));
                    }
                }
                out_row.extend(single_pixel_term(TRANSPARENT));
            }
            out_row.push(b'\n');
            out_row
        })
        .collect::<Vec<u8>>();

    ret.extend(
        format!(
            "\x02{}\x02 by \x02{:?}\x02; created {}.\n",
            moose.name, moose.author, moose.created
        )
        .as_bytes(),
    );
    ret
}

fn idx_1dto2d(x: usize, y: usize, width: usize) -> usize {
    x + y * width
}

pub fn moose_png(moose: &Moose) -> Result<Vec<u8>, png::EncodingError> {
    // 4KiB
    let mut cursor = std::io::Cursor::new(Vec::with_capacity(4096usize));
    {
        let (dim_x, dim_y, total) = moose.dimensions.width_height();
        let mut encoder = png::Encoder::new(
            &mut cursor,
            (PIX_FMT_WIDTH * dim_x) as u32,
            (PIX_FMT_HEIGHT * dim_y) as u32,
        );
        encoder.set_compression(png::Compression::Best);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_palette(&PLTE[..]);
        encoder.set_trns(&TRNS[..]);
        let mut writer = encoder.write_header()?;

        let mut bitmap = vec![0x99u8; total * PIX_FMT_HEIGHT * PIX_FMT_WIDTH];
        for (idx, &pixel) in moose.image.iter().enumerate() {
            let base_y = (idx / dim_x) * PIX_FMT_HEIGHT;
            let base_x = (idx % dim_x) * PIX_FMT_WIDTH;

            // Each pixel needs to occupy a (w * h) square around its position.
            for y in base_y..(base_y + PIX_FMT_HEIGHT) {
                for x in base_x..(base_x + PIX_FMT_WIDTH) {
                    bitmap[idx_1dto2d(x, y, PIX_FMT_WIDTH * dim_x)] = pixel;
                }
            }
        }

        writer.write_image_data(&bitmap)?;
    }
    Ok(cursor.into_inner())
}
