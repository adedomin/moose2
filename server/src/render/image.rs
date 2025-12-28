/* Copyright (C) 2025  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use miniz_oxide::deflate::CompressionLevel;

use crate::{
    model::{
        PIX_FMT_HEIGHT, PIX_FMT_WIDTH,
        color::{EXTENDED_COLORS, RGBA, TRANSPARENT},
        moose::Moose,
    },
    render::helpers::trim_moose,
};

const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const TRNS: &[u8] = &[
    0, 0, 0, 0x01, 0x74, 0x52, 0x4E, 0x53, 0, 0x40, 0xE6, 0xD8, 0x66,
];
const IEND: &[u8] = &[0, 0, 0, 0, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];

/// To reduce the size of the PLTE, we recolor the moose with only the colors actually used.
fn recolor_image(mut image: Vec<&[u8]>) -> (Vec<Vec<u8>>, Vec<u8>) {
    let mut cmap = [0; EXTENDED_COLORS.len()];
    // find used colors
    image
        .iter()
        .flat_map(|row| row.iter())
        .for_each(|pix| cmap[*pix as usize] = 1);
    // filter out unused
    let mut palette = cmap
        .iter()
        .enumerate()
        .filter(|(_, pix)| **pix == 1)
        .map(|(i, _)| i as u8)
        .collect::<Vec<u8>>();
    let pallen = palette.len();
    if pallen > 1 {
        // move transparent to front.
        // so our tRNS can be one byte.
        // if it isn't a transparency color, it doesn't matter.
        palette.swap(0, pallen - 1);
    }
    // now reverse map our new palette.
    palette
        .iter()
        .enumerate()
        .for_each(|(i, pix)| cmap[*pix as usize] = i as u8);
    // recolor pixels
    let nimage = image
        .drain(..)
        .map(|row| row.iter().map(|pix| cmap[*pix as usize]).collect())
        .collect();
    (nimage, palette)
}

/// helper function that maps a 2D (x, y) coordinate to a 1D array.
fn idx_1dto2d(x: usize, y: usize, width: usize) -> usize {
    x + y * width
}

/// helper to generate every coordinate in a grid.
fn xyrange(sx: usize, ex: usize, sy: usize, ey: usize) -> impl Iterator<Item = (usize, usize)> {
    (sy..ey).flat_map(move |j| (sx..ex).map(move |i| (i, j)))
}

/// Generate the uncompressed PNG bitmap.
fn draw_bitmap(image: &[Vec<u8>], dim_x: usize, dim_y: usize) -> Vec<u8> {
    let width = PIX_FMT_WIDTH * dim_x;
    let filter_width = width + 1;
    let height = PIX_FMT_HEIGHT * dim_y;
    let mut bitmap = std::vec::from_elem(
        0,
        dim_x * dim_y * PIX_FMT_WIDTH * PIX_FMT_HEIGHT + height, /* filter bits */
    );
    xyrange(0, dim_x, 0, dim_y)
        .flat_map(|(x, y)| {
            let pixel = image[y][x];
            let base_y = y * PIX_FMT_HEIGHT;
            let base_x = x * PIX_FMT_WIDTH;
            // filter bit...
            (base_y..base_y + PIX_FMT_HEIGHT)
                .map(move |y| (idx_1dto2d(base_x, y, filter_width) + 1, pixel))
        })
        .for_each(|(idx, pixel)| bitmap[idx..idx + PIX_FMT_WIDTH].fill(pixel));
    bitmap
}

fn gen_plte(mut palette: Vec<u8>) -> Vec<u8> {
    palette
        .drain(..)
        .flat_map(|pix| {
            let RGBA(r, g, b, _) = EXTENDED_COLORS[pix as usize];
            [r, g, b]
        })
        .collect()
}

const IHDR: &[u8] = b"IHDR";
const IHDR_SIZ: u32 = 13;
const EIGHT_BPP: u8 = 8;
const PALETTE_TYPE: u8 = 0x3;
const PLTE: &[u8] = b"PLTE";
const IDAT: &[u8] = b"IDAT";

pub fn draw_png(w: u32, h: u32, plte: Vec<u8>, has_trns: bool, zimg: Vec<u8>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4096);
    macro_rules! write_crc {
        ($off:ident) => {
            // note, the CRC does not include the length!
            let crc = crc32fast::hash(&buf[$off + 4..]);
            buf.extend(crc.to_be_bytes());
        };
    }
    buf.extend(PNG_MAGIC);

    let off = buf.len();
    buf.extend(IHDR_SIZ.to_be_bytes());
    buf.extend(IHDR);
    buf.extend(w.to_be_bytes());
    buf.extend(h.to_be_bytes());
    buf.push(EIGHT_BPP); // bit depth
    buf.push(PALETTE_TYPE); // color type
    buf.extend([0, 0, 0]); // unused: commpression method, filter method and interlacing.
    write_crc!(off);

    let off = buf.len();
    buf.extend((plte.len() as u32).to_be_bytes());
    buf.extend(PLTE);
    buf.extend(plte);
    write_crc!(off);

    // there is only one transparent color.
    if has_trns {
        buf.extend(TRNS);
    }

    let off = buf.len();
    buf.extend((zimg.len() as u32).to_be_bytes());
    buf.extend(IDAT);
    buf.extend(zimg);
    write_crc!(off);

    buf.extend(IEND);
    buf
}

/// Given a moose, returns an encoded PNG rendering.
pub fn moose_png(moose: &Moose) -> Vec<u8> {
    let trimmed = trim_moose(&moose.image, &moose.dimensions);
    let (dim_x, dim_y) = trimmed
        .first()
        .map(|row| (row.len(), trimmed.len()))
        .expect("trim_moose always returns at least one pixel.");
    let (image, palette) = recolor_image(trimmed);
    let bitmap = draw_bitmap(&image, dim_x, dim_y);
    let bitmap = miniz_oxide::deflate::compress_to_vec_zlib(
        &bitmap,
        CompressionLevel::BestCompression as u8,
    );
    let trns = palette[0] == TRANSPARENT;
    let plte = gen_plte(palette);

    // Create the PNG
    draw_png(
        (PIX_FMT_WIDTH * dim_x) as u32,
        (PIX_FMT_HEIGHT * dim_y) as u32,
        plte,
        trns,
        bitmap,
    )
}
