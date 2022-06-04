use crate::config;
use chrono::{DateTime, Utc};
use roaring::RoaringBitmap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Read},
    ops::Index,
    path::PathBuf,
    vec::Vec,
};

pub const PAGE_SIZE: usize = 12;

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Author {
    Anonymous,
    Github(String),
    Local(String),
}

/// width, height
pub const DEFAULT_SIZE: (usize, usize, usize) = (26, 15, 26 * 15);
pub const HD_SIZE: (usize, usize, usize) = (36, 22, 36 * 22);
// this is for PNG output, technically the line output is variable based on font x-height
pub const PIX_FMT_WIDTH: usize = 16;
pub const PIX_FMT_HEIGHT: usize = 24;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Dimensions {
    Default,
    HD,
    Custom(usize, usize),
}

impl Dimensions {
    /// Get the width and height of the given dimension.
    pub fn width_height(&self) -> (usize, usize, usize) {
        match self {
            Dimensions::Default => DEFAULT_SIZE,
            Dimensions::HD => HD_SIZE,
            Dimensions::Custom(width, height) => (*width, *height, *width * *height - 1),
        }
    }

    /// Decipher the likely dimensions of a moose by their 1-D Image size.
    pub fn from_len(image: &[u8]) -> Option<Self> {
        if image.len() == DEFAULT_SIZE.2 {
            Some(Self::Default)
        } else if image.len() == HD_SIZE.2 {
            Some(Self::HD)
        } else {
            None
        }
    }
}

impl Default for Dimensions {
    fn default() -> Self {
        Self::Default
    }
}

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

pub struct MoosePage<'m>(pub &'m [Moose]);

impl<'m> From<MoosePage<'m>> for Vec<u8> {
    fn from(meese: MoosePage) -> Self {
        serde_json::to_vec(meese.0).unwrap()
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

#[derive(thiserror::Error, Debug)]
pub enum InsertError {
    #[error("Moose violates constraint: {0}")]
    Conformity(String),
    #[error("JSON Parse error or wrong shape.")]
    JsonError(#[from] serde_json::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum OpenError {
    #[error("IO Issue")]
    IO(#[from] std::io::Error),
    #[error("JSON Parse error or wrong shape.")]
    JsonError(#[from] serde_json::Error),
}

pub struct MooseDb {
    pub meese: Vec<Moose>,
    pub meese_idx: HashMap<String, usize>,
    pub meese_fts: HashMap<String, RoaringBitmap>,
}

impl Index<usize> for MooseDb {
    type Output = Moose;

    fn index(&self, index: usize) -> &Self::Output {
        &self.meese[index]
    }
}

/// Break up every name into parts that consist only of ASCII Alphanumeric characters (values higher than 7bit ignored),
/// then convert to lowercase.
/// Do we care about utf-8 "spaces?" probably not; what about moose with only characters like "/"?
fn search_tokenize_words(s: &str) -> impl Iterator<Item = String> + '_ {
    s.as_bytes()
        .split(|&chr| matches!(chr, 0_u8..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=127_u8))
        // since this is already valid unicode, since we aren't taking out >127 chars, this should be safe.
        .map(|byte_slice| unsafe { std::str::from_utf8_unchecked(byte_slice) })
        .map(|s| s.to_ascii_lowercase())
}

fn reindex_db(meese: &[Moose]) -> (HashMap<String, usize>, HashMap<String, RoaringBitmap>) {
    let meese_idx = meese
        .iter()
        .enumerate()
        .map(|(pos, moose)| (moose.name.clone(), pos))
        .collect::<HashMap<String, usize>>();

    let meese_fts = meese
        .iter()
        .enumerate()
        .flat_map(|(pos, moose)| search_tokenize_words(&moose.name).map(move |s| (s, pos)))
        .fold(
            HashMap::new(),
            |mut acc: HashMap<String, RoaringBitmap>, (word, pos)| {
                if let Some(wordset) = acc.get_mut(&word) {
                    //  my memory would be wasted before this becomes a problem...
                    wordset.insert(pos as u32);
                } else {
                    acc.insert(word, RoaringBitmap::from_iter([pos as u32].iter().cloned()));
                }
                acc
            },
        );

    (meese_idx, meese_fts)
}

impl MooseDb {
    pub fn get(&self, name: &str) -> Option<&Moose> {
        self.meese_idx.get(name).map(|&idx| &self[idx])
    }

    pub fn page_count(&self) -> usize {
        self.meese.len() / PAGE_SIZE
            + if self.meese.len() % PAGE_SIZE > 0 {
                1
            } else {
                0
            }
    }

    pub fn get_page(&self, page_num: usize) -> MoosePage {
        let start = page_num * PAGE_SIZE;
        let end = if start + PAGE_SIZE > self.meese.len() {
            self.meese.len()
        } else {
            start + PAGE_SIZE
        };

        MoosePage(self.meese.get(start..end).unwrap_or(&[] as &[Moose]))
    }

    fn find(&self, query: &str) -> Option<RoaringBitmap> {
        search_tokenize_words(query)
            .flat_map(|word| self.meese_fts.get(&word)) // We're removing words with no hits to the reverse index.... good?
            .cloned()
            .reduce(|acc, next| acc & next)
            .and_then(|result| {
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            })
    }

    pub fn find_page(&self, query: &str) -> Vec<&Moose> {
        self.find(query)
            .map(|bmap| {
                bmap.iter()
                    .take(PAGE_SIZE * 5)
                    .flat_map(|idx| self.meese.get(idx as usize))
                    .collect::<Vec<&Moose>>()
            })
            .unwrap_or_else(Vec::new)
    }

    pub fn find_page_with_link(&self, query: &str) -> Vec<(usize, &Moose)> {
        self.find(query)
            .map(|bmap| {
                bmap.iter()
                    .take(PAGE_SIZE * 5)
                    .flat_map(|idx| {
                        self.meese
                            .get(idx as usize)
                            .map(|moose| (idx as usize / PAGE_SIZE, moose))
                    })
                    .collect::<Vec<(usize, &Moose)>>()
            })
            .unwrap_or_else(Vec::new)
    }

    pub fn find_page_bin(&self, query: &str) -> Vec<u8> {
        serde_json::to_vec(&self.find_page(query)).unwrap()
    }

    pub fn find_page_with_link_bin(&self, query: &str) -> Vec<u8> {
        serde_json::to_vec(&self.find_page_with_link(query)).unwrap()
    }

    pub fn open() -> std::io::Result<Self> {
        let conf = <config::Args as clap::Parser>::parse();
        let mut moose_json = Vec::with_capacity(2usize.pow(21));
        // partial read failure will likely result in invalid json read anyhow.
        let _ = std::fs::File::open(conf.get_moose_path())?.read_to_end(&mut moose_json)?;

        let mut meese = serde_json::from_slice::<Vec<Moose>>(&moose_json)?;
        // make sure they are ordered by date
        meese.sort_by(|moose_l, moose_r| moose_l.created.cmp(&moose_r.created));
        let (meese_idx, meese_fts) = reindex_db(&meese);

        Ok(MooseDb {
            meese,
            meese_idx,
            meese_fts,
        })
    }

    pub fn insert_moose(&self, moose: &[u8]) -> Result<(), InsertError> {
        let mut new_moose = serde_json::from_slice::<Moose>(moose)?;
        // Always override insert date
        new_moose.created = Utc::now();
        // for Full-Text Searching and cleaning up moose names
        let words = new_moose
            .name
            .split_ascii_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let new_name = words.join(" ");
        new_moose.name = new_name;
        Ok(())
    }
}
