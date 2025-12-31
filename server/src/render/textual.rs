use crate::{
    model::{
        color::{EXTENDED_COLORS, RGBA, TRANSPARENT},
        moose::Moose,
    },
    render::helpers::{reladate, trim_moose},
};

const IRC_BOLD: &str = "\x02";
const IRC_LINE_END: &[u8] = b"\n";

fn pix_char_irc(pixel: u8) -> u8 {
    if pixel == TRANSPARENT { b' ' } else { b'@' }
}

fn single_pixel_irc(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        b"\x03 ".to_vec()
    } else {
        format!("\x03{0},{0}@", pixel).into()
    }
}

const TERM_BOLD: &str = "\x1b[1m";
const TERM_BOLD_END: &str = "\x1b[0m";
const TERM_LINE_END: &[u8] = b"\x1b[0m\n";

fn pix_char_term(_pixel: u8) -> u8 {
    b' '
}

fn single_pixel_term(pixel: u8) -> Vec<u8> {
    if pixel == TRANSPARENT {
        b"\x1b[0m ".to_vec()
    } else {
        let RGBA(r, g, b, _) = EXTENDED_COLORS[pixel as usize];
        format!("\x1b[48;2;{r};{g};{b}m ").into()
    }
}

fn format_info(moose: &Moose, bold_start: &'static str, bold_end: &'static str) -> String {
    use std::fmt::Write as _;
    let mut ret = String::new();
    write!(&mut ret, "{bold_start}{}{bold_end}", moose.name).unwrap();
    if let Some(disp) = moose.author.clone().displayable() {
        write!(&mut ret, " by {bold_start}{disp}{bold_end}").unwrap();
    }
    if moose.upvotes > 0 {
        write!(&mut ret, " \u{2bc5}{}", moose.upvotes).unwrap();
    }
    writeln!(&mut ret, " created {}", reladate(&moose.created)).unwrap();
    ret
}

macro_rules! impl_line {
    ($fn_name:ident, $pix_char_fn:ident, $single_pix_fn:ident, $line_end:ident, $bold_start:ident, $bold_end:ident) => {
        pub fn $fn_name(moose: &Moose) -> Vec<u8> {
            let mut ret = vec![];
            trim_moose(&moose.image, &moose.dimensions)
                .into_iter()
                .for_each(|row| {
                    let mut last_pix = TRANSPARENT;
                    for &pix in row {
                        if pix == last_pix {
                            ret.push($pix_char_fn(pix));
                        } else {
                            last_pix = pix;
                            ret.extend($single_pix_fn(pix));
                        }
                    }
                    ret.extend($line_end);
                });
            ret.extend(format_info(moose, $bold_start, $bold_end).as_bytes());
            ret
        }
    };
}

impl_line!(
    moose_irc,
    pix_char_irc,
    single_pixel_irc,
    IRC_LINE_END,
    IRC_BOLD,
    IRC_BOLD
);
impl_line!(
    moose_term,
    pix_char_term,
    single_pixel_term,
    TERM_LINE_END,
    TERM_BOLD,
    TERM_BOLD_END
);
