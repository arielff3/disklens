//! Saves each scan to `%LOCALAPPDATA%\Disklens\index\` so the app reopens instantly.
//!
//! File layout: a header that never changes between versions (magic, format
//! version, root path, scan time), then the zstd-compressed bincode body. Because
//! the header is stable, even a file we can't read still says which root to rescan.

use crate::index::{Index, Node};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Bump whenever `Node` or `Body` changes shape.
pub const FORMAT_VERSION: u32 = 1;
const MAGIC: &[u8; 4] = b"DLIX";
const EXTENSION: &str = "dlx";
const MAX_ROOT_LEN: usize = 32 * 1024;
const ZSTD_LEVEL: i32 = 3;
const BUFFER: usize = 1 << 20;

pub struct Store {
    dir: PathBuf,
}

#[derive(Debug)]
pub struct Loaded {
    pub index: Index,
    /// Unix seconds when the scan finished.
    pub scanned_at: i64,
}

/// A snapshot that couldn't be used. Its file has already been deleted.
#[derive(Debug)]
pub struct Discarded {
    /// The root it belonged to, if the header was readable.
    pub root: Option<String>,
    pub reason: String,
}

#[derive(Serialize)]
struct BodyRef<'a> {
    unreadable_dirs: u64,
    nodes: &'a [Node],
}

#[derive(Deserialize)]
struct Body {
    unreadable_dirs: u64,
    nodes: Vec<Node>,
}

struct Header {
    version: u32,
    root: String,
    scanned_at: i64,
}

fn bincode_config() -> impl bincode::config::Config {
    // The limit keeps a corrupt length prefix from allocating absurd amounts.
    bincode::config::standard().with_limit::<{ 1 << 34 }>()
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Store {
        Store { dir: dir.into() }
    }

    /// `%LOCALAPPDATA%\Disklens\index`, or `DISKLENS_INDEX_DIR` when set
    /// (keeps test runs away from real snapshots).
    pub fn default_dir() -> Option<PathBuf> {
        if let Some(dir) = std::env::var_os("DISKLENS_INDEX_DIR") {
            return Some(PathBuf::from(dir));
        }
        Some(dirs::data_local_dir()?.join("Disklens").join("index"))
    }

    /// Where the snapshot for `root` lives. Same file for any spelling of the path.
    pub fn path_for(&self, root: &str) -> PathBuf {
        let key = root_key(root);
        let mut prefix: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        prefix = prefix.trim_matches('_').chars().take(40).collect();
        self.dir.join(format!("{prefix}-{:016x}.{EXTENSION}", fnv1a(key.as_bytes())))
    }

    /// Writes the index atomically (temp file, then rename). Returns the file path.
    pub fn save(&self, index: &Index, scanned_at: i64) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        let root = index.path_of(index.root());
        let path = self.path_for(&root);
        let tmp = path.with_extension("tmp");
        let result = (|| {
            let mut out = BufWriter::with_capacity(BUFFER, File::create(&tmp)?);
            write_header(&mut out, &Header { version: FORMAT_VERSION, root, scanned_at })?;
            let mut encoder = zstd::stream::write::Encoder::new(out, ZSTD_LEVEL)?;
            encoder.include_checksum(true)?;
            let mut body = BufWriter::with_capacity(BUFFER, encoder);
            let nodes = index.compacted();
            let value = BodyRef { unreadable_dirs: index.unreadable_dirs(), nodes: &nodes };
            bincode::serde::encode_into_std_write(&value, &mut body, bincode_config()).map_err(io::Error::other)?;
            let encoder = body.into_inner().map_err(|e| e.into_error())?;
            encoder.finish()?.into_inner().map_err(|e| e.into_error())?.sync_all()
        })();
        match result {
            Ok(()) => fs::rename(&tmp, &path).map(|_| path),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }

    /// Loads the snapshot for `root`. `Ok(None)` if there is none; an unusable
    /// file is deleted and reported as [`Discarded`].
    pub fn load(&self, root: &str) -> Result<Option<Loaded>, Discarded> {
        let path = self.path_for(root);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(discard(&path, None, e.to_string())),
        };
        let mut input = BufReader::with_capacity(BUFFER, file);
        let header = match read_header(&mut input) {
            Ok(h) => h,
            Err(e) => return Err(discard(&path, None, e.to_string())),
        };
        let known_root = Some(header.root.clone());
        if header.version != FORMAT_VERSION {
            let reason = format!("format version {} (expected {FORMAT_VERSION})", header.version);
            return Err(discard(&path, known_root, reason));
        }
        if root_key(&header.root) != root_key(root) {
            return Err(discard(&path, known_root, "snapshot belongs to another root".into()));
        }
        match read_body(input) {
            Ok(index) => Ok(Some(Loaded { index, scanned_at: header.scanned_at })),
            Err(e) => Err(discard(&path, known_root, e.to_string())),
        }
    }

    /// Root of the most recently written snapshot. Files whose header can't be
    /// read are deleted and skipped.
    pub fn latest(&self) -> Option<String> {
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&self.dir)
            .ok()?
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == EXTENSION))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        for (_, path) in files {
            let header = File::open(&path).and_then(|f| read_header(&mut BufReader::new(f)));
            match header {
                Ok(h) => return Some(h.root),
                Err(e) => {
                    discard(&path, None, e.to_string());
                }
            }
        }
        None
    }
}

fn discard(path: &Path, root: Option<String>, reason: String) -> Discarded {
    let _ = fs::remove_file(path);
    Discarded { root, reason }
}

fn write_header(out: &mut impl Write, h: &Header) -> io::Result<()> {
    out.write_all(MAGIC)?;
    out.write_all(&h.version.to_le_bytes())?;
    out.write_all(&(h.root.len() as u32).to_le_bytes())?;
    out.write_all(h.root.as_bytes())?;
    out.write_all(&h.scanned_at.to_le_bytes())
}

fn read_header(input: &mut impl Read) -> io::Result<Header> {
    let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
    let mut magic = [0u8; 4];
    input.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(bad("not a Disklens snapshot"));
    }
    let mut u32_buf = [0u8; 4];
    input.read_exact(&mut u32_buf)?;
    let version = u32::from_le_bytes(u32_buf);
    input.read_exact(&mut u32_buf)?;
    let len = u32::from_le_bytes(u32_buf) as usize;
    if len == 0 || len > MAX_ROOT_LEN {
        return Err(bad("bad root length"));
    }
    let mut root = vec![0u8; len];
    input.read_exact(&mut root)?;
    let root = String::from_utf8(root).map_err(|_| bad("root is not UTF-8"))?;
    let mut i64_buf = [0u8; 8];
    input.read_exact(&mut i64_buf)?;
    Ok(Header { version, root, scanned_at: i64::from_le_bytes(i64_buf) })
}

fn read_body(input: impl Read) -> io::Result<Index> {
    let decoder = zstd::stream::read::Decoder::new(input)?;
    let mut body = BufReader::with_capacity(BUFFER, decoder);
    let value: Body =
        bincode::serde::decode_from_std_read(&mut body, bincode_config()).map_err(io::Error::other)?;
    // Reading to the end makes zstd verify the frame checksum.
    let mut rest = Vec::new();
    body.read_to_end(&mut rest)?;
    if !rest.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "trailing data"));
    }
    Index::from_parts(value.nodes, value.unreadable_dirs)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "nodes don't form a tree"))
}

/// Lowercase, backslashes, no trailing separator except in a drive root.
fn root_key(root: &str) -> String {
    let mut s = root.trim().replace('/', "\\").to_lowercase();
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    if s.len() == 2 && s.ends_with(':') {
        s.push('\\');
    }
    s
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{DirTree, FileRec};
    use std::fs;

    fn file(name: &str, size: u64, alloc: u64) -> FileRec {
        FileRec { name: name.into(), size, alloc, modified: 1_700_000_000 }
    }

    fn sample(root: &str) -> Index {
        let locked = DirTree { name: "Locked".into(), unreadable: true, ..Default::default() };
        let users = DirTree {
            name: "Users".into(),
            modified: 42,
            files: vec![file("b.jpg", 10, 4096), file("c.txt", 5000, 8192)],
            dirs: vec![locked],
            ..Default::default()
        };
        Index::build(DirTree {
            name: root.into(),
            files: vec![file("a.mp4", 100, 4096)],
            dirs: vec![users],
            ..Default::default()
        })
    }

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("index"));
        (dir, store)
    }

    #[test]
    fn round_trips_an_index() {
        let (_dir, store) = store();
        let index = sample(r"C:\");
        store.save(&index, 1234).unwrap();
        let loaded = store.load(r"C:\").unwrap().expect("snapshot missing");
        assert_eq!(loaded.scanned_at, 1234);
        assert_eq!(loaded.index.outline(), index.outline());
        assert_eq!(loaded.index.unreadable_dirs(), 1);
    }

    #[test]
    fn missing_snapshot_is_none() {
        let (_dir, store) = store();
        assert!(store.load(r"D:\").unwrap().is_none());
        assert_eq!(store.latest(), None);
    }

    #[test]
    fn drops_tombstones_and_compacts_ids() {
        let (_dir, store) = store();
        let mut index = sample(r"C:\");
        let users = index.find_path(r"C:\Users").unwrap();
        index.remove(index.find_path(r"C:\Users\b.jpg").unwrap());
        index.add_file(users, file("new.zip", 9000, 12288));
        store.save(&index, 1).unwrap();
        let loaded = store.load(r"C:\").unwrap().unwrap().index;
        assert_eq!(loaded.outline(), index.outline());
        assert_eq!(loaded.len(), index.len() - 1);
        assert!(loaded.nodes().iter().all(|n| !n.removed));
    }

    #[test]
    fn latest_is_the_most_recently_saved_root() {
        let (_dir, store) = store();
        store.save(&sample(r"C:\"), 1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        store.save(&sample(r"D:\Games"), 2).unwrap();
        assert_eq!(store.latest().as_deref(), Some(r"D:\Games"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        store.save(&sample(r"C:\"), 3).unwrap();
        assert_eq!(store.latest().as_deref(), Some(r"C:\"));
    }

    #[test]
    fn file_names_are_per_root_and_case_insensitive() {
        let (_dir, store) = store();
        assert_eq!(store.path_for(r"C:\"), store.path_for(r"c:\"));
        assert_eq!(store.path_for(r"C:\Users\"), store.path_for("c:/users"));
        assert_ne!(store.path_for(r"C:\"), store.path_for(r"D:\"));
    }

    #[test]
    fn other_format_version_is_discarded_but_root_is_known() {
        let (_dir, store) = store();
        let path = store.save(&sample(r"C:\"), 1).unwrap();
        let mut bytes = fs::read(&path).unwrap();
        bytes[4..8].copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
        fs::write(&path, bytes).unwrap();
        let err = store.load(r"C:\").unwrap_err();
        assert_eq!(err.root.as_deref(), Some(r"C:\"));
        assert!(!path.exists(), "discarded file should be deleted");
    }

    #[test]
    fn corrupt_body_is_discarded() {
        let (_dir, store) = store();
        let path = store.save(&sample(r"C:\"), 1).unwrap();
        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 10;
        bytes[last] ^= 0xff;
        fs::write(&path, &bytes).unwrap();
        assert_eq!(store.load(r"C:\").unwrap_err().root.as_deref(), Some(r"C:\"));
        assert!(!path.exists());

        let path = store.save(&sample(r"C:\"), 1).unwrap();
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
        assert!(store.load(r"C:\").is_err());
        assert!(!path.exists());
    }

    #[test]
    fn garbage_file_is_discarded_and_skipped_by_latest() {
        let (_dir, store) = store();
        let path = store.path_for(r"C:\");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not a snapshot").unwrap();
        assert_eq!(store.latest(), None);
        assert!(!path.exists());

        fs::write(&path, b"not a snapshot").unwrap();
        assert_eq!(store.load(r"C:\").unwrap_err().root, None);
        assert!(!path.exists());
    }

    #[test]
    fn overwriting_keeps_one_file_per_root() {
        let (dir, store) = store();
        store.save(&sample(r"C:\"), 1).unwrap();
        store.save(&sample(r"C:\"), 2).unwrap();
        let files = fs::read_dir(dir.path().join("index")).unwrap().count();
        assert_eq!(files, 1);
        assert_eq!(store.load(r"C:\").unwrap().unwrap().scanned_at, 2);
    }
}
