/* Copyright 2026 Anthony DeDominic
 *
 * Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

pub use include_static_macro::include_static;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct StaticContent {
    pub path: &'static str,
    pub content: &'static [u8],
    pub etag: &'static str,
    pub mime: &'static str,
}

pub type StaticContents = &'static [StaticContent];

pub const fn const_eq_str(lhs: &[u8], rhs: &[u8]) -> bool {
    if lhs.len() != rhs.len() {
        return false;
    }

    let mut i = 0;
    while i < lhs.len() {
        if lhs[i] != rhs[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub const fn find_static_by_path(
    contents: StaticContents,
    path: &str,
) -> Option<&'static StaticContent> {
    if contents.is_empty() {
        return None;
    }

    let path_bytes = path.as_bytes();
    let mut i = 0;
    while i < contents.len() {
        let item = &contents[i];
        if const_eq_str(path_bytes, item.path.as_bytes()) {
            return Some(item);
        }
        i += 1;
    }
    None
}
