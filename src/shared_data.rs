use crate::render::{EXTENDED_COLORS, RGBA};

const COLOR_PREAMBLE: &[u8] = b"export default [";
const COLOR_END: &[u8] = b"];\n";
// 12 = '#XXXXXXXX',
const SOURCE_LEN: usize = EXTENDED_COLORS.len() * 12 + COLOR_PREAMBLE.len() + COLOR_END.len();

macro_rules! const_write_bytes {
    ($target:ident, $source:expr, $off:ident) => {
        let source = $source;
        let mut i = 0usize;
        while i < source.len() {
            $target[i + $off] = source[i];
            i += 1;
        }
        $off += i;
    };
}

const fn hex(c: u8) -> u8 {
    match c {
        0..=9 => b'0' + c,
        0xA..=0xF => b'a' + c - 10u8,
        _ => 0,
    }
}

const fn hex_color(RGBA(r, g, b, a): RGBA) -> [u8; 8] {
    let mut ret = [b'0'; 8];
    ret[0] = hex(r >> 4);
    ret[1] = hex(r & 0b1111);
    ret[2] = hex(g >> 4);
    ret[3] = hex(g & 0b1111);
    ret[4] = hex(b >> 4);
    ret[5] = hex(b & 0b1111);
    ret[6] = hex(a >> 4);
    ret[7] = hex(a & 0b1111);
    ret
}

pub const COLORS_JS: [u8; SOURCE_LEN] = {
    let mut module_file: [u8; SOURCE_LEN] = [b' '; SOURCE_LEN];
    let mut i = 0usize;
    let mut off = 0usize;

    const_write_bytes!(module_file, COLOR_PREAMBLE, off);

    while i < EXTENDED_COLORS.len() {
        const_write_bytes!(module_file, b"'#", off);
        const_write_bytes!(module_file, hex_color(EXTENDED_COLORS[i]), off);
        const_write_bytes!(module_file, b"',", off);
        i += 1;
    }

    const_write_bytes!(module_file, COLOR_END, off);
    module_file
};
