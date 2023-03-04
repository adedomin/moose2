use phf::phf_map;

pub const MIME: phf::Map<&'static str, &'static str> = phf_map! {
    "css" => "text/css; charset=utf-8",
    "html" => "text/html; charset=utf-8",
    "js" => "application/javascript; charset=utf-8",
    "wasm" => "application/wasm",
    "ico" => "image/x-icon",
};
