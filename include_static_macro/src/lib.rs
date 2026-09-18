/* Copyright 2026 Anthony DeDominic
 *
 * Permission to use, copy, modify, and/or distribute this software for
 * any purpose with or without fee is hereby granted, provided that the
 * above copyright notice and this permission notice appear in all
 * copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL
 * WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE
 * AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL
 * DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR
 * PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
 * TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
 * PERFORMANCE OF THIS SOFTWARE.
 */

use std::{ffi::OsStr, fs, path::PathBuf};

use base64::Engine as _;
use quote::quote;
use sha2::Digest;

#[derive(PartialOrd, Ord, PartialEq, Eq)]
struct Ent(String, String, String, &'static str);

const MIME: [(&str, &str); 5] = [
    ("css", "text/css; charset=utf-8"),
    ("html", "text/html; charset=utf-8"),
    ("js", "application/javascript; charset=utf-8"),
    ("wasm", "application/wasm"),
    ("ico", "image/x-icon"),
];

fn get_supported_mime(ext: &OsStr) -> &'static str {
    match MIME.iter().find(|(e, _)| ext == *e) {
        Some((_, t)) => t,
        None => "application/octet-string",
    }
}

#[proc_macro]
pub fn include_static(tok: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut tok_itr = tok.into_iter();
    let tok = tok_itr.next().expect("Need one arg.");
    assert!(tok_itr.next().is_none(), "Only one string allowed");
    let mut path = tok.to_string();
    if path.starts_with('"') {
        _ = path.remove(0);
    }
    if path.ends_with('"') {
        _ = path.pop();
    }
    if path.is_empty() {
        panic!("Path lenght cannot be empty.");
    }
    let root = root.join(path);

    let mut files: Vec<Ent> = vec![];
    let mut dfs = vec![root.clone()];
    while let Some(path) = dfs.pop() {
        if path.is_dir() {
            let ents = fs::read_dir(&path)
                .unwrap_or_else(|_| panic!("expected to read dir: {path:?}"))
                .collect::<Result<Vec<_>, std::io::Error>>()
                .unwrap_or_else(|_| panic!("to open all files in a directory {path:?}"));
            for item in ents {
                let path = item.path();
                dfs.push(path);
            }
        } else if path.is_file() {
            let bin = fs::read(&path).expect("to open/read file");
            // 28 bytes. used for caching.
            let hash = sha2::Sha224::digest(bin);
            // should be smaller than hex encode.
            let hash = base64::engine::general_purpose::STANDARD_NO_PAD.encode(hash);
            let etag = format!("\"{hash}\"");
            let new_path = path.strip_prefix(&root).expect("huh?");
            let mime = new_path
                .extension()
                .map(get_supported_mime)
                .unwrap_or("application/octet-string");
            let to_str = new_path.to_string_lossy();
            #[cfg(windows)]
            let to_str = to_str.replace('\\', "/");

            files.push(Ent(
                to_str.into_owned(),
                path.to_string_lossy().into_owned(),
                etag,
                mime,
            ));
        }
    }
    // note that having the same file twice would be... weird.
    files.sort_unstable();

    let files_iter = files.into_iter().map(|Ent(path, real_path, etag, mime)| {
        quote! {
            StaticContent {
                path: #path,
                content: include_bytes!(#real_path),
                etag: #etag,
                mime: #mime,
            }
        }
    });
    quote! {
        &[#(#files_iter),*]
    }
    .into()
}
