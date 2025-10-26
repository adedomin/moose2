/* Copyright (C) 2024  Anthony DeDominic
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

use super::color::{SHADE_TO_EXTENDED, SHADE_TRNS};
use super::dimensions::Dimensions;
use super::{author::Author, color::TRANSPARENT};
use base64::{DecodeError, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    io::{BufReader, BufWriter},
    path::PathBuf,
};
use time::{
    OffsetDateTime, PrimitiveDateTime, format_description::FormatItem, macros::format_description,
};

const MOOSE_MAX_NAME_LEN: usize = 64usize;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(remote = "Self")]
pub struct Moose {
    #[serde(deserialize_with = "control_len_bound_string")]
    pub name: String,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    pub image: Vec<u8>,
    pub dimensions: Dimensions,
    #[serde(serialize_with = "as_js", deserialize_with = "from_js")]
    pub created: OffsetDateTime,
    #[serde(default = "super::author::default_author")]
    pub author: Author,
    #[serde(default = "upvote_zeroed")]
    pub upvotes: i64,
}

fn upvote_zeroed() -> i64 {
    0
}

impl Serialize for Moose {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Moose {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize(deserializer).and_then(|moose| {
            // You're probably wondering why we remote = "Self" our Deserializer
            // we do it because Dimensions is directly tied to image and without
            // twiddling the structure of the type, we have to fully deserialize
            // to validate the dimensions match the image.
            match moose.dimensions {
                Dimensions::Custom(w, h) => {
                    if moose.image.len() != w * h {
                        return Err(serde::de::Error::custom(
                            "Moose.image length does not match Moose.dimensions.",
                        ));
                    }
                }
                Dimensions::Default => {
                    if !matches!(
                        Dimensions::from_len(&moose.image),
                        Some(Dimensions::Default),
                    ) {
                        return Err(serde::de::Error::custom(
                            "Moose.image length is not correct.",
                        ));
                    }
                }
                Dimensions::HD => {
                    if !matches!(Dimensions::from_len(&moose.image), Some(Dimensions::HD)) {
                        return Err(serde::de::Error::custom(
                            "Moose.image length is not an HD moose.",
                        ));
                    }
                }
            }

            Ok(moose)
        })
    }
}

fn control_len_bound_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    String::deserialize(deserializer).and_then(|name| {
        if name.is_empty() {
            return Err(serde::de::Error::custom("Moose.name is empty."));
        }

        if name.len() > MOOSE_MAX_NAME_LEN {
            return Err(serde::de::Error::custom(
                "Moose.name is too long: >64 bytes.",
            ));
        }

        if matches!(name.as_str(), "random" | "latest" | "oldest") {
            return Err(serde::de::Error::custom(
                "Moose.name cannot be a reserved word: random | latest | oldest",
            ));
        }

        if name.contains(|chr| matches!(chr, '\x00'..='\x1f')) {
            return Err(serde::de::Error::custom(
                "Moose.name cannot contain an ASCII control character.",
            ));
        }

        if name != name.trim() {
            return Err(serde::de::Error::custom(
                "Moose.name cannot contain leading/trailing whitespace.",
            ));
        }

        Ok(name)
    })
}

fn as_base64<S: Serializer>(image: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(image))
}

fn from_base64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    String::deserialize(deserializer).and_then(|string| {
        base64::engine::general_purpose::STANDARD
            .decode(string)
            .and_then(|decoded| {
                if let Some(pos) = decoded.iter().position(
                    |&b| b > TRANSPARENT, /* anything bigger than Transparent is invalid */
                ) {
                    Err(DecodeError::InvalidByte(pos, decoded[pos]))
                } else {
                    Ok(decoded)
                }
            })
            .map_err(|err| serde::de::Error::custom(err.to_string()))
    })
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MooseLegacy {
    pub name: String,
    pub image: String,
    pub shade: String,
    #[serde(serialize_with = "as_js", deserialize_with = "from_js")]
    pub created: OffsetDateTime,
    pub hd: bool,
    pub shaded: bool,
    pub extended: bool,
}

pub fn truncate_to(s: &str, max_len: usize) -> &str {
    if max_len >= s.len() {
        return s;
    }
    let mut idx = max_len;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

impl From<MooseLegacy> for Moose {
    fn from(old: MooseLegacy) -> Self {
        let new_image: Vec<u8> = if old.extended {
            old.image
                .bytes()
                .zip(old.shade.bytes())
                .flat_map(|(color, shade)| extended_color_code(color, shade))
                .collect()
        } else if old.shaded {
            old.image
                .bytes()
                .zip(old.shade.bytes())
                .flat_map(|(color, shade)| shaded_color_code(color, shade))
                .map(|color| SHADE_TO_EXTENDED[color as usize])
                .collect()
        } else {
            old.image.bytes().flat_map(parse_hexish_opt).collect()
        };

        let dimensions =
            Dimensions::from_len(&new_image).expect("expected moose to be HD or default size.");

        let name = truncate_to(&old.name, MOOSE_MAX_NAME_LEN)
            .trim()
            .to_string();

        Moose {
            name,
            image: new_image,
            dimensions,
            created: old.created,
            author: Author::Anonymous,
            upvotes: 0,
        }
    }
}

impl From<&Moose> for Vec<u8> {
    fn from(moose: &Moose) -> Self {
        serde_json::to_vec(moose).unwrap()
    }
}

impl From<Moose> for Vec<u8> {
    fn from(moose: Moose) -> Self {
        serde_json::to_vec(&moose).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MooseAny {
    Moose(Moose),
    MooseLegacy(MooseLegacy),
}

impl From<MooseAny> for Moose {
    fn from(variant: MooseAny) -> Self {
        match variant {
            MooseAny::Moose(m) => m,
            MooseAny::MooseLegacy(ml) => ml.into(),
        }
    }
}

pub fn moose_bulk_transform(moose_in: Option<PathBuf>, moose_out: Option<PathBuf>) {
    let mut moose_in = match moose_in {
        Some(path) => {
            let file = BufReader::new(std::fs::File::open(path).unwrap());
            serde_json::from_reader::<_, Vec<MooseLegacy>>(file).unwrap()
        }
        None => serde_json::from_reader::<_, Vec<MooseLegacy>>(std::io::stdin().lock()).unwrap(),
    };
    let meese = moose_in
        .drain(..)
        .map(|legacy| legacy.into())
        .collect::<Vec<Moose>>();
    match moose_out {
        Some(path) => {
            let file = BufWriter::new(std::fs::File::create(path).unwrap());
            serde_json::to_writer::<_, Vec<Moose>>(file, &meese).unwrap()
        }
        None => serde_json::to_writer::<_, Vec<Moose>>(std::io::stdout().lock(), &meese).unwrap(),
    };
}

/// Turn Meese string into byte values.
/// use flat_map and make sure to map dimension by explicitly defining it.
fn parse_hexish(hex: u8) -> u8 {
    match hex {
        b'0'..=b'9' => hex - 48,
        b'a'..=b'f' => (hex - 97) + 10,
        b'A'..=b'F' => (hex - 65) + 10,
        b't' => TRANSPARENT,
        // invalid color, including \n
        _ => 100,
    }
}

fn parse_hexish_opt(hex: u8) -> Option<u8> {
    let phex = parse_hexish(hex);
    if phex == 100 { None } else { Some(phex) }
}

/// Legacy Moose shaded color value to u8
/// note shade transparency repeats for each shade
/// we convert these by mapping them against SHADE_TO_EXTENDED anyway.
fn shaded_color_code(color: u8, shade: u8) -> Option<u8> {
    if color == b't' || shade == b't' {
        Some(SHADE_TRNS)
    } else if color == b'\n' && shade == b'\n' {
        None
    } else {
        match (parse_hexish_opt(color), parse_hexish_opt(shade)) {
            (Some(color), Some(shade)) => Some(1 + color + (17 * shade)),
            _ => None,
        }
    }
}

/// IRC Extended Color Code -> 0..=99
/// optional because newlines do not belong in our data.
/// use flat_map and make sure to map dimension by explicitly defining it.
fn extended_color_code(color: u8, shade: u8) -> Option<u8> {
    if color == b't' && shade == b't' {
        Some(TRANSPARENT)
    } else if color == b'\n' && shade == b'\n' {
        None
    } else {
        match (parse_hexish_opt(color), parse_hexish_opt(shade)) {
            (Some(color), Some(shade)) => Some(16 + color + (12 * shade)),
            _ => None,
        }
    }
}

/// SEE: https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-date-time-string-format
/// It's a simplified ISO-8601.
/// YYYY is mandatory.
/// MM-DD or DD are optional.
/// ss.mmm or ss are optional
/// mmm must be exactly 3 significant figures. It's representing milliseconds, not the full 9 sig fig nanoseconds.
/// we only ever care about UTC (Z) timezone and should reject any other offsets.
///
/// NOTE: this has been highly simplified due to unfortunate pains with format_description!, issues with OffsetDateTime deserialization.
const JS_DATE_TIME_FORMAT: &[FormatItem<'_>] = format_description!(
    version = 2,
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
);

fn as_js<S: Serializer>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(
        datetime
            .format(JS_DATE_TIME_FORMAT)
            .map_err(serde::ser::Error::custom)?
            .as_str(),
    )
}

fn from_js<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OffsetDateTime, D::Error> {
    String::deserialize(deserializer).and_then(|string| {
        PrimitiveDateTime::parse(&string, JS_DATE_TIME_FORMAT)
            .map(|parsed| parsed.assume_utc())
            .map_err(serde::de::Error::custom)
    })
}
