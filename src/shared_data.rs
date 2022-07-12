use crate::{
    moosedb::{DEFAULT_SIZE, HD_SIZE, PIX_FMT_HEIGHT, PIX_FMT_WIDTH},
    render::{EXTENDED_COLORS, RGBA},
};

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
        // ....the block is for this annotation.
        #[allow(unused_assignments)]
        {
            $off += i;
        }
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
    let mut off = 0usize;
    const_write_bytes!(module_file, COLOR_PREAMBLE, off);

    let mut i = 0usize;
    while i < EXTENDED_COLORS.len() {
        const_write_bytes!(module_file, b"'#", off);
        const_write_bytes!(module_file, hex_color(EXTENDED_COLORS[i]), off);
        const_write_bytes!(module_file, b"',", off);
        i += 1;
    }

    const_write_bytes!(module_file, COLOR_END, off);
    module_file
};

const fn usize_to_u16hex(num: usize) -> [u8; 6] {
    let mut ret = [0; 6];
    ret[0] = b'0';
    ret[1] = b'x';

    let num = num & (0xFFFF);
    ret[2] = hex((num >> 12) as u8);
    ret[3] = hex(((num & 0x0FFF) >> 8) as u8);
    ret[4] = hex(((num & 0x00FF) >> 4) as u8);
    ret[5] = hex((num & 0x000F) as u8);
    ret
}

const SIZ_NAMED_FIELD_1: &[u8] = b"const PIX_FMT_WIDTH=";
const SIZ_NAMED_FIELD_2: &[u8] = b";const PIX_FMT_HEIGHT=";
const SIZ_NAMED_FIELD_3: &[u8] = b";const MOOSE_SIZES=new Map([";
const SIZ_END: &[u8] = b"]);export {PIX_FMT_WIDTH,PIX_FMT_HEIGHT,MOOSE_SIZES};\n";
const SIZ_LEN: usize = 6 * 8 // size of number (0x0000) and the number of them (6)
    + 13 // extra syntax [,] for MOOSE_SIZES field.
    + SIZ_END.len()
    + SIZ_NAMED_FIELD_1.len()
    + SIZ_NAMED_FIELD_2.len()
    + SIZ_NAMED_FIELD_3.len();
pub const SIZ_JS: [u8; SIZ_LEN] = {
    let mut module_file = [b' '; SIZ_LEN];
    let mut off = 0usize;

    const_write_bytes!(module_file, SIZ_NAMED_FIELD_1, off);
    const_write_bytes!(module_file, usize_to_u16hex(PIX_FMT_WIDTH), off);

    const_write_bytes!(module_file, SIZ_NAMED_FIELD_2, off);
    const_write_bytes!(module_file, usize_to_u16hex(PIX_FMT_HEIGHT), off);

    const_write_bytes!(module_file, SIZ_NAMED_FIELD_3, off);
    // for visual aid only
    {
        let (small_w, small_h, small_l) = DEFAULT_SIZE;
        module_file[off] = b'[';
        off += 1;
        const_write_bytes!(module_file, usize_to_u16hex(small_l), off);
        const_write_bytes!(module_file, b",[", off);
        const_write_bytes!(module_file, usize_to_u16hex(small_w), off);
        module_file[off] = b',';
        off += 1;
        const_write_bytes!(module_file, usize_to_u16hex(small_h), off);
        const_write_bytes!(module_file, b"]],[", off);

        let (hd_w, hd_h, hd_l) = HD_SIZE;
        const_write_bytes!(module_file, usize_to_u16hex(hd_l), off);
        const_write_bytes!(module_file, b",[", off);
        const_write_bytes!(module_file, usize_to_u16hex(hd_w), off);
        module_file[off] = b',';
        off += 1;
        const_write_bytes!(module_file, usize_to_u16hex(hd_h), off);
        const_write_bytes!(module_file, b"]]", off);
    }
    const_write_bytes!(module_file, SIZ_END, off);

    module_file
};
