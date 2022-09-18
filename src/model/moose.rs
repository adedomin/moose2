use super::other::{Author, Dimensions};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{io::{BufWriter, BufReader}, path::PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Moose {
    pub name: String,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    pub image: Vec<u8>,
    pub dimensions: Dimensions,
    pub created: DateTime<Utc>,
    pub author: Author,
}

fn as_base64<S: Serializer>(image: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::encode(image))
}

fn from_base64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    String::deserialize(deserializer).and_then(|string| {
        base64::decode(string).map_err(|err| serde::de::Error::custom(err.to_string()))
    })
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MooseLegacy {
    pub name: String,
    pub image: String,
    pub shade: String,
    pub created: DateTime<Utc>,
    pub hd: bool,
    pub shaded: bool,
    pub extended: bool,
}

impl From<MooseLegacy> for Moose {
    fn from(old: MooseLegacy) -> Self {
        if old.shaded {
            panic!("CONVERT TO EXTENDED PALETTE FIRST USING https://github.com/adedomin/moose `moose import < moose`");
        }
        let new_image: Vec<u8> = if old.extended {
            old.image
                .bytes()
                .zip(old.shade.bytes())
                .flat_map(|(pix, shade)| extended_color_code(pix, shade))
                .collect()
        } else {
            old.image.bytes().flat_map(parse_hexish_opt).collect()
        };

        let dimensions =
            Dimensions::from_len(&new_image).expect("expected moose to be HD or default size.");

        Moose {
            name: old.name,
            image: new_image,
            dimensions,
            created: old.created,
            author: Author::Anonymous,
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
        b't' => 99,
        // invalid color, including \n
        _ => 100,
    }
}

fn parse_hexish_opt(hex: u8) -> Option<u8> {
    let phex = parse_hexish(hex);
    if phex == 100 {
        None
    } else {
        Some(phex)
    }
}

/// IRC Extended Color Code -> 0..=99
/// optional because newlines do not belong in our data.
/// use flat_map and make sure to map dimension by explicitly defining it.
fn extended_color_code(color: u8, shade: u8) -> Option<u8> {
    if color == b't' && shade == b't' {
        Some(99u8)
    } else if color == b'\n' && shade == b'\n' {
        None
    } else {
        match (parse_hexish_opt(color), parse_hexish_opt(shade)) {
            (Some(color), Some(shade)) => Some(16 + color + (12 * shade)),
            _ => None,
        }
    }
}
