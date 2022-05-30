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

const PAGE_SIZE: usize = 12;

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Moose {
    pub name: String,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    pub image: Vec<u8>,
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
fn parse_hexish(hex: u8) -> u8 {
    match hex {
        b'0'..=b'9' => hex - 48,
        b'a'..=b'f' => hex - (97 - 9),
        b'A'..=b'F' => hex - (65 - 9),
        b't' => 16,
        b'\n' => 17,
        _ => 0,
    }
}

/// IRC Extended Color Code -> 0..=99
fn extended_color_code(color: u8, shade: u8) -> u8 {
    16 + parse_hexish(color) + (12 * parse_hexish(shade))
}

impl From<MooseLegacy> for Moose {
    fn from(old: MooseLegacy) -> Self {
        if old.shaded {
            panic!("CONVERT TO EXTENDED PALETTE FIRST USING https://github.com/adedomin/moose `moose import < moose`");
        }
        let new_image = if old.extended {
            old.image
                .bytes()
                .zip(old.shade.bytes())
                .map(|(pix, shade)| extended_color_code(pix, shade))
                .collect()
        } else {
            old.image.bytes().map(parse_hexish).collect()
        };
        Moose {
            name: old.name,
            image: new_image,
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

fn reindex_db(meese: &[Moose]) -> (HashMap<String, usize>, HashMap<String, RoaringBitmap>) {
    let meese_idx = meese
        .iter()
        .enumerate()
        .map(|(pos, moose)| (moose.name.clone(), pos))
        .collect::<HashMap<String, usize>>();

    let meese_fts = meese
        .iter()
        .enumerate()
        .flat_map(|(pos, moose)| {
            moose
                .name
                .split_ascii_whitespace()
                .map(|s| (s.to_ascii_lowercase(), pos))
                .collect::<Vec<(String, usize)>>()
        })
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
    pub fn get(&self, name: &str) -> Option<Moose> {
        self.meese_idx.get(name).map(|&idx| self[idx].clone())
    }

    pub fn get_bin(&self, name: &str) -> Option<Vec<u8>> {
        self.get(name)
            .map(|moose| serde_json::to_vec(&moose).unwrap())
    }

    pub fn page_count(&self) -> usize {
        self.meese.len() / PAGE_SIZE
    }

    pub fn get_page(&self, page_num: usize) -> Vec<Moose> {
        let start = page_num * PAGE_SIZE;
        let end = if start + PAGE_SIZE > self.meese.len() {
            self.meese.len()
        } else {
            start + PAGE_SIZE
        };
        self.meese
            .get(start..end)
            .map(|slice| slice.to_vec())
            .unwrap_or_else(Vec::new)
    }

    pub fn get_page_bin(&self, page_num: usize) -> Vec<u8> {
        let meese = self.get_page(page_num);
        serde_json::to_vec::<Vec<Moose>>(&meese).unwrap()
    }

    fn find(&self, query: &str) -> Option<RoaringBitmap> {
        query
            .split_ascii_whitespace()
            .map(|word| word.to_ascii_lowercase())
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

    pub fn find_page(&self, query: &str) -> Vec<Moose> {
        self.find(query)
            .map(|bmap| {
                bmap.iter()
                    .flat_map(|idx| self.meese.get(idx as usize))
                    .cloned()
                    .collect::<Vec<Moose>>()
            })
            .unwrap_or_else(Vec::new)
    }

    pub fn find_page_bin(&self, query: &str) -> Vec<u8> {
        serde_json::to_vec::<Vec<Moose>>(&self.find_page(query)).unwrap()
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
