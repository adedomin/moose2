use crate::{
    model::{
        color::{EXTENDED_COLORS, RGBA, TRANSPARENT},
        moose::Moose,
    },
    render::helpers::{reladate, trim_moose},
};
use std::io::Write;

enum LineType {
    IrcArt,
    TrueColorTerm,
}

pub fn pix_char(pixel: u8) -> u8 {
    if pixel == TRANSPARENT { b' ' } else { b'@' }
}

const IRC_BOLD: &str = "\x02";
const TERM_BOLD: &str = "\x1b[1m";
const TERM_END: &str = "\x1b[0m";

pub fn single_pixel_term(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        b"\x1b[0m ".to_vec()
    } else {
        let RGBA(r, g, b, _) = EXTENDED_COLORS[pixel as usize];
        format!("\x1b[48;2;{r};{g};{b}m ").into()
    }
}

pub fn single_pixel(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        vec![b'\x03', b' ']
    } else {
        format!("\x03{0},{0}{1}", pixel, pix_char(pixel) as char).into()
    }
}

fn moose_line(moose: &Moose, l: LineType) -> Vec<u8> {
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

    let (bstart, bend) = match l {
        LineType::IrcArt => (IRC_BOLD, IRC_BOLD),
        LineType::TrueColorTerm => (TERM_BOLD, TERM_END),
    };
    write!(&mut ret, "{bstart}{}{bend}", moose.name).unwrap();
    if let Some(disp) = moose.author.clone().displayable() {
        write!(&mut ret, " by {bstart}{disp}{bend}").unwrap();
    }
    writeln!(&mut ret, " created {}", reladate(&moose.created)).unwrap();
    ret
}

pub fn moose_irc(moose: &Moose) -> Vec<u8> {
    moose_line(moose, LineType::IrcArt)
}

pub fn moose_term(moose: &Moose) -> Vec<u8> {
    moose_line(moose, LineType::TrueColorTerm)
}
