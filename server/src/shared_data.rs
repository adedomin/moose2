use crate::{
    model::{dimensions::DEFAULT_SIZE, dimensions::HD_SIZE, PIX_FMT_HEIGHT, PIX_FMT_WIDTH},
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

pub const SIZ_JS: &[u8] = const_format::formatcp!(
    r###"
const PIX_FMT_WIDTH = {};
const PIX_FMT_HEIGHT = {};
const MOOSE_SIZES = new Map([
    [{}, [{}, {}]],
    [{}, [{}, {}]],
]);
export {{PIX_FMT_WIDTH, PIX_FMT_HEIGHT, MOOSE_SIZES}};
"###,
    PIX_FMT_WIDTH,
    PIX_FMT_HEIGHT,
    DEFAULT_SIZE.2, // length
    DEFAULT_SIZE.0, // width
    DEFAULT_SIZE.1, // height
    HD_SIZE.2,
    HD_SIZE.0,
    HD_SIZE.1,
)
.as_bytes();

pub const EXAMPLE_CONFIG: &[u8] = br###"{ "//": "OPTIONAL: default: $XDG_DATA_HOME/moose2 or $STATE_DIRECTORY/"
, "moose_path":    "/path/to/store/meese"
, "//": "OPTIONAL: can use unix:/path/to/socket for uds listening."
, "listen":        "http://[::1]:5921"
, "//": "A symmetric secret key for session cookies; delete for random; is PBKDF padded to 64 bytes."
, "cookie_secret": "super-duper-sekret"
, "//": "github oauth2 client configuration details, omit whole object to disable authentication."
, "github_oauth2": { "id":     "client id"
                   , "secret": "client secret"
                   }
}
"###;

pub const ERR_JS: &[u8] = br###"window.onerror = function (event, source, lineno, colno, error) {
  console.log(event, source, lineno, colno, error);
  console.log('JS error, attempting to fallback to nojs.');
  var x = window.location.origin;
  var y = window.location.pathname;
  window.location = x + y + '?nojs=true';
};"###;
