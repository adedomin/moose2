/* Copyright (C) 2024  Anthony DeDominic
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

use crate::model::{
    PIX_FMT_HEIGHT, PIX_FMT_WIDTH,
    color::{COLOR_MAP_SIGIL, EXTENDED_COLORS, RGBA, TRANSPARENT},
    dimensions::Dimensions,
    moose::Moose,
};
use std::cmp::Ordering::{Equal, Greater, Less};

fn pix_char(pixel: u8) -> u8 {
    if pixel == TRANSPARENT { b' ' } else { b'@' }
}

fn single_pixel_term(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        b"\x1b[0m ".to_vec()
    } else {
        let RGBA(r, g, b, _) = EXTENDED_COLORS[pixel as usize];
        format!("\x1b[48;2;{r};{g};{b}m ").into()
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
    // this is focused trimming the top and bottoms of the frame.
    let partials = image
        .chunks_exact(dim_x)
        .skip_while(is_same) // skip over all the rows that are transparent at start.
        .collect::<Vec<&'m [u8]>>()
        .into_iter()
        .rev() // now repeat, but from the bottom
        .skip_while(is_same)
        .collect::<Vec<&'m [u8]>>()
        .into_iter()
        .rev() // now flip again to restore original orientation.
        .collect::<Vec<&'m [u8]>>();

    if let Some((left_trim, right_trim)) = partials
        .iter()
        .map(|row| {
            let left = row
                .iter()
                .take_while(|&&pixel| pixel == TRANSPARENT)
                .count(); // how many leading transparents.
            let right = row
                .iter()
                .rev()
                .take_while(|&&pixel| pixel == TRANSPARENT)
                .count(); // how many trailing transparents.
            (left, right)
        })
        // now we find the smallest common leading and trailing transparency (if any).
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
        // now remove the leading / trailing
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
            moose.name,
            moose.author,
            moose
                .created
                .format(&time::format_description::well_known::Rfc2822)
                .unwrap_or_else(|e| {
                    log::error!("time claimed formatting the timestamp failed {e}");
                    "(TIME FORMAT ERROR)".to_owned()
                })
        )
        .as_bytes(),
    );
    ret
}

/// To reduce the size of the PLTE, we first select only the colors our moose has.
/// this map serves as the proto PLTE.
fn gen_color_map(image: &[&[u8]]) -> [u8; EXTENDED_COLORS.len()] {
    image
        .iter()
        .flat_map(|row| row.iter())
        .fold(
            (
                [COLOR_MAP_SIGIL; EXTENDED_COLORS.len()],
                0u8,
                COLOR_MAP_SIGIL,
            ),
            |(mut colmap, curr, zeroth), &pix| {
                debug_assert!(curr < COLOR_MAP_SIGIL);
                if colmap[pix as usize] != COLOR_MAP_SIGIL {
                    return (colmap, curr, zeroth);
                }

                colmap[pix as usize] = curr;
                let zeroth = if curr == 0 { pix } else { zeroth };
                // only one color (99) is "transparent"
                // the tRNS segment does not need to be complete, weirdly enough
                // so we can just move the transparent color to the front and make trns = 1 "tRNS" 0 CRC
                if pix == TRANSPARENT {
                    colmap.swap(TRANSPARENT as usize, zeroth as usize);
                }

                (colmap, curr + 1, zeroth)
            },
        )
        .0
}

/// helper function that maps a 2D (x, y) coordinate to a 1D array.
fn idx_1dto2d(x: usize, y: usize, width: usize) -> usize {
    x + y * width
}

/// helper to generate every coordinate in a grid.
fn xyrange(sx: usize, ex: usize, sy: usize, ey: usize) -> impl Iterator<Item = (usize, usize)> {
    (sy..ey).flat_map(move |j| (sx..ex).map(move |i| (i, j)))
}

/// Generate the moose bitmap.
fn draw_bitmap(
    image: &[&[u8]],
    color_map: &[u8; EXTENDED_COLORS.len()],
    dim_x: usize,
    dim_y: usize,
    total: usize,
) -> Vec<u8> {
    let width = PIX_FMT_WIDTH * dim_x;
    let mut bitmap = vec![0x99u8; total * PIX_FMT_WIDTH * PIX_FMT_HEIGHT];
    xyrange(0, dim_x, 0, dim_y)
        .flat_map(|(x, y)| {
            let pixel = image[y][x];
            let pixel = color_map[pixel as usize];
            let base_y = y * PIX_FMT_HEIGHT;
            let base_x = x * PIX_FMT_WIDTH;
            (base_y..base_y + PIX_FMT_HEIGHT).map(move |y| (idx_1dto2d(base_x, y, width), pixel))
        })
        .for_each(|(idx, pixel)| bitmap[idx..idx + PIX_FMT_WIDTH].fill(pixel));
    bitmap
}

// Generate the PLTE (palette) from our color_map.
fn gen_plte(color_map: [u8; EXTENDED_COLORS.len()]) -> ([u8; EXTENDED_COLORS.len() * 3], usize) {
    let mut plte = [00u8; EXTENDED_COLORS.len() * 3];
    let mut len = 0usize;
    color_map
        .into_iter()
        .enumerate()
        .filter(|&(_, nidx)| nidx < COLOR_MAP_SIGIL)
        .for_each(|(cidx, nidx)| {
            let i = nidx as usize * 3usize;

            plte[i] = EXTENDED_COLORS[cidx].0;
            plte[i + 1] = EXTENDED_COLORS[cidx].1;
            plte[i + 2] = EXTENDED_COLORS[cidx].2;
            len += 1;
        });
    (plte, len)
}

/// Given a moose, returns an encoded PNG rendering.
pub fn moose_png(moose: &Moose) -> Result<Vec<u8>, png::EncodingError> {
    // 4KiB
    let mut cursor = std::io::Cursor::new(Vec::with_capacity(4096usize));
    {
        let mut trimmed = trim_moose(&moose.image, &moose.dimensions);
        let (dim_x, dim_y, total) = trimmed
            .first()
            .map(|row| (row.len(), trimmed.len(), row.len() * trimmed.len()))
            .unwrap_or_else(|| {
                // PNGs must contain at least one pixel.
                trimmed = vec![&[0]];
                (1, 1, 1)
            });
        let color_map = gen_color_map(&trimmed);
        let bitmap = draw_bitmap(&trimmed, &color_map, dim_x, dim_y, total);
        let trns = color_map[TRANSPARENT as usize] != COLOR_MAP_SIGIL;
        let (plte, plte_len) = gen_plte(color_map);
        let plte = &plte[..plte_len * 3];

        // Create the PNG
        let mut encoder = png::Encoder::new(
            &mut cursor,
            (PIX_FMT_WIDTH * dim_x) as u32,
            (PIX_FMT_HEIGHT * dim_y) as u32,
        );
        encoder.set_compression(png::Compression::High);
        encoder.set_filter(png::Filter::NoFilter);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_palette(plte);
        // the tRNS segment does not need to be map each palette color
        // when generating the color_map & PLTE above, we make sure the only
        // transparent character is index 0 in the PLTE
        if trns {
            encoder.set_trns(&[0u8]);
        }
        encoder.write_header()?.write_image_data(&bitmap)?;
    }
    Ok(cursor.into_inner())
}
