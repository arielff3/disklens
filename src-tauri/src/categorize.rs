//! Maps file names to the coarse categories shown in the breakdown bar.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Video,
    Images,
    Audio,
    Archives,
    Code,
    Documents,
    Apps,
    DataCaches,
    Other,
}

pub const CATEGORY_COUNT: usize = 9;

impl Category {
    /// Every category, in the fixed order used to break ties when sorting.
    pub const ALL: [Category; CATEGORY_COUNT] = [
        Category::Video,
        Category::Images,
        Category::Audio,
        Category::Archives,
        Category::Code,
        Category::Documents,
        Category::Apps,
        Category::DataCaches,
        Category::Other,
    ];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Categorizes a file by its extension. Names without one, and dotfiles, are `Other`.
pub fn categorize(file_name: &str) -> Category {
    let ext = match file_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => return Category::Other,
    };
    match ext.as_str() {
        "mp4" | "mkv" | "mov" | "avi" | "wmv" | "webm" | "m4v" | "flv" | "mpg" | "mpeg"
        | "mts" | "m2ts" | "3gp" => Category::Video,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "heif" | "tif" | "tiff"
        | "raw" | "cr2" | "cr3" | "nef" | "arw" | "dng" | "psd" | "svg" | "ico" | "avif" => {
            Category::Images
        }
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff" => {
            Category::Audio
        }
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "iso" | "img"
        | "cab" | "dmg" | "xip" | "vhd" | "vhdx" | "wim" | "esd" => Category::Archives,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "pyc" | "java" | "class"
        | "jar" | "kt" | "c" | "h" | "cpp" | "hpp" | "cc" | "cs" | "go" | "rb" | "php"
        | "swift" | "m" | "scala" | "lua" | "sh" | "ps1" | "bat" | "cmd" | "html" | "css"
        | "scss" | "vue" | "svelte" | "sql" | "toml" | "yaml" | "yml" | "json" | "xml" | "md"
        | "pdb" | "obj" | "o" | "a" | "lib" | "rlib" | "rmeta" | "d" => Category::Code,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "txt" | "rtf" | "csv" | "epub" => Category::Documents,
        "exe" | "dll" | "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" | "sys"
        | "node" | "so" | "dylib" | "apk" => Category::Apps,
        "db" | "sqlite" | "sqlite3" | "ldb" | "log" | "tmp" | "temp" | "cache" | "dat"
        | "bin" | "pak" | "blob" | "etl" | "dmp" | "mdmp" | "bak" | "old" | "idx" | "pack"
        | "wal" | "journal" | "lock" => Category::DataCaches,
        _ => Category::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_extensions_case_insensitively() {
        assert_eq!(categorize("holiday.MP4"), Category::Video);
        assert_eq!(categorize("photo.heic"), Category::Images);
        assert_eq!(categorize("song.flac"), Category::Audio);
        assert_eq!(categorize("Win11.iso"), Category::Archives);
        assert_eq!(categorize("main.rs"), Category::Code);
        assert_eq!(categorize("index.ts"), Category::Code);
        assert_eq!(categorize("report.pdf"), Category::Documents);
        assert_eq!(categorize("setup.exe"), Category::Apps);
        assert_eq!(categorize("History.sqlite"), Category::DataCaches);
        assert_eq!(categorize("thing.xyz"), Category::Other);
    }

    #[test]
    fn names_without_extension_are_other() {
        assert_eq!(categorize("README"), Category::Other);
        assert_eq!(categorize(".gitignore"), Category::Other);
        assert_eq!(categorize("trailing."), Category::Other);
    }

    #[test]
    fn index_matches_position_in_all() {
        for (i, c) in Category::ALL.iter().enumerate() {
            assert_eq!(c.index(), i);
        }
    }
}
