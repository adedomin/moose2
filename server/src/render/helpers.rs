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

use time::OffsetDateTime;

use crate::model::{color::TRANSPARENT, dimensions::Dimensions};

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
    let month = (year * 12) - (moose_date.month() as i32) + (current.month() as i32);
    let day = current.to_julian_day() - moose_date.to_julian_day();
    if day == 0 {
        "Today".to_owned()
    } else if day < i32::from(current.month().length(current.year())) {
        fmt_diff_str!(day)
    } else if month < 12 {
        fmt_diff_str!(month)
    } else {
        // TODO: do I want to show something like "8 years, 10 months ago." ?
        let year = month / 12;
        fmt_diff_str!(year)
    }
}

pub fn trim_moose<'m>(image: &'m [u8], dim: &Dimensions) -> Vec<&'m [u8]> {
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
