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

#[cfg(feature = "sha2sum")]
use std::fs::path::Path;
use std::{ffi::OsStr, fs, path::PathBuf};

#[cfg(feature = "sha2sum")]
use base64::Engine as _;
use proc_macro2::TokenStream;
use quote::quote;
#[cfg(feature = "sha2sum")]
use sha2::Digest;

#[derive(PartialOrd, Ord, PartialEq, Eq)]
struct Ent {
    p: String,
    rp: String,
    #[cfg(feature = "sha2sum")]
    hash: String,
    mime: &'static str,
}

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

#[cfg(feature = "sha2sum")]
fn map_ent(Ent { p, rp, hash, mime }: Ent) -> TokenStream {
    quote! {
        StaticContent {
            path: #p,
            content: include_bytes!(#rp),
            hash: #hash,
            mime: #mime,
        }
    }
}

#[cfg(not(feature = "sha2sum"))]
fn map_ent(Ent { p, rp, mime }: Ent) -> TokenStream {
    quote! {
        StaticContent {
            path: #p,
            content: include_bytes!(#rp),
            mime: #mime,
        }
    }
}

#[cfg(feature = "sha2sum")]
fn get_file_hash(p: &Path) -> String {
    let bin = fs::read(path).expect("to open/read file");
    let hash = sha2::Sha224::digest(bin);
    // should be smaller than hex encode.
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(hash)
}

fn parse_arg_to_path(tok: proc_macro::TokenStream) -> PathBuf {
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

    root.join(path)
}

#[proc_macro]
pub fn include_static(tok: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let root = parse_arg_to_path(tok);

    let mut files: Vec<Ent> = vec![];
    let mut dfs = vec![root.clone()];
    while let Some(path) = dfs.pop() {
        if path.is_dir() {
            fs::read_dir(&path)
                .unwrap_or_else(|_| panic!("expected to read dir: {path:?}"))
                .try_for_each(|ent| -> Result<(), std::io::Error> {
                    let path = ent?.path();
                    dfs.push(path);
                    Ok(())
                })
                .unwrap_or_else(|_| panic!("Failed to read all files in {path:?}."));
        } else if path.is_file() {
            // 28 bytes. used for caching.
            #[cfg(feature = "sha2sum")]
            let hash = get_file_hash(&path);

            let macro_path = path.strip_prefix(&root).expect("huh?");
            let macro_path = macro_path.to_string_lossy().into_owned();
            #[cfg(windows)]
            let macro_path = macro_path.replace('\\', "/");

            let mime = path
                .extension()
                .map(get_supported_mime)
                .unwrap_or("application/octet-string");

            files.push(Ent {
                p: macro_path,
                rp: path.to_string_lossy().into_owned(),
                #[cfg(feature = "sha2sum")]
                hash,
                mime,
            });
        }
    }

    // note that having the same file twice would be... weird.
    files.sort_unstable();
    let files_iter = files.into_iter().map(map_ent);
    quote! { &[#(#files_iter),*] }.into()
}
