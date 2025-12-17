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

use time::OffsetDateTime;

use crate::model::{
    PIX_FMT_HEIGHT, PIX_FMT_WIDTH,
    color::{COLOR_MAP_SIGIL, EXTENDED_COLORS, RGBA, TRANSPARENT},
    dimensions::Dimensions,
    moose::Moose,
};

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

fn trim_moose<'m>(image: &'m [u8], dim: &Dimensions) -> Vec<&'m [u8]> {
    let dim_x = dim.width_height().0;
    // break image up into rows.
    let image = image.chunks_exact(dim_x).collect::<Vec<&'m [u8]>>();
    // remove all "Transparent" lines from the top.
    let top_trim = image
        .iter()
        .take_while(|row| row.iter().all(|&p| p == TRANSPARENT))
        .count();
    // empty image.
    // return an image with one pixel.
    if top_trim == image.len() {
        return vec![&[0u8]];
    }
    // from bottom..
    let bottom_trim = image
        .iter()
        .rev()
        .take_while(|row| row.iter().all(|&p| p == TRANSPARENT))
        .count();
    // trim vert.
    let partial = &image[top_trim..(image.len() - bottom_trim)];
    // we should always have at least one row when here.
    let (left_trim, right_trim) = partial
        .iter()
        .fold((usize::MAX, usize::MAX), |(l, r), row| {
            // from left
            let ll = row.iter().take_while(|&&p| p == TRANSPARENT).count();
            // from right
            let rr = row.iter().rev().take_while(|&&p| p == TRANSPARENT).count();
            // must take the minimum to not trim content on other rows.
            (l.min(ll), r.min(rr))
        });
    // trim hori, return.
    partial
        .iter()
        .map(|row| &row[left_trim..(row.len() - right_trim)])
        .collect()
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

pub fn reladate(moose_date: &OffsetDateTime) -> String {
    macro_rules! fmt_diff_str {
        ($diff:ident) => {{
            let plural = if $diff != 1 { "s" } else { "" };
            return format!("{} {}{plural} ago", $diff, stringify!($diff));
        }};
    }
    let current = time::OffsetDateTime::now_utc();
    if moose_date > &current {
        return "In the future.".to_owned();
    }
    let year = current.year() - moose_date.year();
    let month = (year * 12) - (moose_date.month() as i32 - 1) + (current.month() as i32 - 1);
    let day = current.to_julian_day() - moose_date.to_julian_day();
    if day == 0 {
        "Today".to_owned()
    } else if day < i32::from(current.month().length(current.year())) {
        fmt_diff_str!(day)
    } else if month < 12 {
        fmt_diff_str!(month)
    } else {
        fmt_diff_str!(year)
    }
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
            reladate(&moose.created),
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
        let trimmed = trim_moose(&moose.image, &moose.dimensions);
        let (dim_x, dim_y, total) = trimmed
            .first()
            .map(|row| (row.len(), trimmed.len(), row.len() * trimmed.len()))
            .expect("trim_moose always returns at least one pixel.");
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
