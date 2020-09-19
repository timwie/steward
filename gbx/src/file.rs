use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use anyhow::{anyhow, bail, ensure};
use byteorder::{ByteOrder, LittleEndian};

use crate::DisplayString;

/// Selected information stored in the header of a `*.Map.Gbx` file.
#[derive(Debug)]
pub struct MapFileHeader {
    pub uid: String,
    pub name: DisplayString,
    pub millis_bronze: i32,
    pub millis_silver: i32,
    pub millis_gold: i32,
    pub millis_author: i32,
    pub is_multilap: bool,
    pub author_login: String,
    pub author_display_name: DisplayString,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ChunkName {
    Info,
    String,
    Version,
    XML,
    Thumbnl,
    Author,
}

#[derive(Debug)]
struct ChunkInfo {
    offset: usize,
    size: i32,
}

/// Parse a `*.Map.Gbx` file at the given path.
///
/// Reference:
/// - https://wiki.xaseco.org/wiki/GBX
/// - https://forum.maniaplanet.com/viewtopic.php?t=14421
#[allow(unused_assignments)] // warns that 'i' is never read, which is obviously not true
pub fn parse_map_file<P: AsRef<Path>>(path: P) -> anyhow::Result<MapFileHeader> {
    let mut f = std::fs::File::open(&path)?;
    let metadata = std::fs::metadata(&path)?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read_exact(&mut buffer)?;

    let mut i = 0;
    let mut chunks = HashMap::new();
    let mut chunk_lookback_strings = Vec::<&str>::new();

    macro_rules! read_bytes {
        ($len:expr) => {{
            let res = &buffer[i..i + $len];
            i += $len;
            res
        }};
    }

    macro_rules! read_i8 {
        () => {{
            let res = buffer[i];
            i += 1;
            res as i8
        }};
    }

    macro_rules! read_i16 {
        () => {{
            let bytes = read_bytes!(2);
            LittleEndian::read_i16(&bytes)
        }};
    }

    macro_rules! read_i32 {
        () => {{
            let bytes = read_bytes!(4);
            LittleEndian::read_i32(&bytes)
        }};
    }

    macro_rules! read_str {
        () => {{
            let mut str_len = read_i32!() as usize;
            str_len &= 0x7FFFFFFF;
            read_str!(str_len)
        }};
        ($len:expr) => {{
            let bytes = read_bytes!($len);
            std::str::from_utf8(&bytes)?
        }};
    }

    macro_rules! read_lookback_str {
        () => {{
            if chunk_lookback_strings.is_empty() {
                let version = read_i32!();
                ensure!(version == 3, "unknown lookback strings version");
            }

            let index = read_i32!();
            if index == -1 {
                ""
            } else if (index as u32 & 0xC0000000) == 0 {
                match index {
                    26 => "Stadium",
                    _ => bail!("unknown external reference string"),
                }
            } else if index.trailing_zeros() >= 30 {
                let str = read_str!();
                chunk_lookback_strings.push(str.clone());
                str
            } else {
                let index = (index & 0x3FFFFFFF) - 1;
                chunk_lookback_strings[index as usize]
            }
        }};
    }

    macro_rules! move_to_chunk {
        ($chunk_name:expr) => {{
            i = match chunks.get(&$chunk_name) {
                Some(info) => info.offset,
                None => bail!("missing chunk"),
            };

            // the lookback string state is reset after each header chunk.
            chunk_lookback_strings.clear();
        }};
    }

    // === Header ===

    let magic = read_str!(3);
    ensure!(magic == "GBX", "no magic header");

    let version = read_i16!();
    ensure!(version == 6, "unknown header version");

    let _ = read_bytes!(4); // skip format/compression/unknown bytes

    let main_class_id = read_i32!();
    const GBX_CHALLENGE_TMF: i32 = 0x03043000;
    ensure!(main_class_id == GBX_CHALLENGE_TMF, "not a map file");

    let header_size = read_i32!();

    let num_chunks = read_i32!();

    let chunk_start = i;
    let mut chunk_offset = chunk_start + num_chunks as usize * 8;

    for _chunk_idx in 0..num_chunks {
        let chunk_id = read_i32!();

        let mut chunk_size = read_i32!();
        chunk_size &= 0x7FFFFFFF;

        let chunk_name = match chunk_id {
            0x03043002 => ChunkName::Info,
            0x03043003 => ChunkName::String,
            0x03043004 => ChunkName::Version,
            0x03043005 => ChunkName::XML,
            0x03043007 => ChunkName::Thumbnl,
            0x03043008 => ChunkName::Author,
            _ => return Err(anyhow!("unexpected chunk id")),
        };

        let chunk = ChunkInfo {
            offset: chunk_offset,
            size: chunk_size,
        };
        chunks.insert(chunk_name, chunk);

        chunk_offset += chunk_size as usize;
    }

    let total_size = chunk_offset - chunk_start + 4;
    ensure!(
        header_size as usize == total_size,
        "content size doesn't match header"
    );

    // === "Info" chunk ===

    move_to_chunk!(ChunkName::Info);
    let chunk_version = read_i32!();
    ensure!(chunk_version >= 13, "Info chunk version < 13");

    let _ = read_bytes!(4); // skip bool 0

    let millis_bronze = read_i32!();
    let millis_silver = read_i32!();
    let millis_gold = read_i32!();
    let millis_author = read_i32!();
    let _cost = read_i32!();
    let is_multilap = match read_i32!() {
        0 => false,
        _ => true,
    };
    let _type = read_i32!();

    let _ = read_bytes!(4); // skip int32 0

    let _editor_mode = read_i32!();

    let _ = read_bytes!(4); // skip bool 0

    let _nb_cps = read_i32!();
    let _nb_laps = read_i32!();

    // === "String" chunk ===

    move_to_chunk!(ChunkName::String);
    let chunk_version = read_i8!();
    ensure!(chunk_version >= 11, "String chunk version < 11");

    let uid = read_lookback_str!().to_string();
    let _envi = read_lookback_str!();
    let _author = read_lookback_str!();
    let name = DisplayString::from(read_str!().to_string());
    let _kind = read_i8!();

    let _ = read_bytes!(4); // skip locked

    let _password = read_str!();
    let _mood = read_lookback_str!();
    let _envi_bg = read_lookback_str!();
    let _author_bg = read_lookback_str!();

    let _ = read_bytes!(8); // skip mapTarget
    let _ = read_bytes!(8); // skip mapOrigin
    let _ = read_bytes!(16); // skip unknown int128

    let _type = read_str!();
    let _style = read_str!();

    let _ = read_bytes!(8); // skip lightmapCacheUID
    let _lightmap = read_i8!();
    let _title_uid = read_lookback_str!();

    // === "Author" chunk ===

    move_to_chunk!(ChunkName::Author);
    let chunk_version = read_i32!();
    ensure!(chunk_version == 1, "unknown author chunk version");

    let _author_version = read_i32!();

    let author_login = read_str!().to_string();
    let author_display_name = DisplayString::from(read_str!().to_string());
    let _author_zone = read_str!().to_string();
    let _author_extra_info = read_str!().to_string();

    Ok(MapFileHeader {
        uid,
        name,
        millis_bronze,
        millis_silver,
        millis_gold,
        millis_author,
        is_multilap,
        author_login,
        author_display_name,
    })
}
