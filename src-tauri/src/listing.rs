//! Lettura dell'output tecnico di `7z l -slt`.

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub packed: u64,
    pub modified: String,
    pub is_dir: bool,
    pub encrypted: bool,
    pub symlink: Option<String>,
    pub hardlink: Option<String>,
    /// Link simbolico il cui bersaglio non è leggibile dall'elenco (es. ZIP Unix).
    pub link_unknown_target: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub format: String,
    pub physical_size: u64,
    pub headers_encrypted: bool,
    pub entries: Vec<Entry>,
}

impl Listing {
    pub fn total_size(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum()
    }
    pub fn any_encrypted(&self) -> bool {
        self.headers_encrypted || self.entries.iter().any(|e| e.encrypted)
    }
}

/// Formati che comprimono un singolo flusso: se contengono un `.tar`
/// l'archivio va trattato come tarball compresso (due livelli).
pub const STREAM_FORMATS: &[&str] = &[
    "gzip", "bzip2", "xz", "zstd", "lzma", "z", "lz4", "lzip", "brotli",
];

impl Listing {
    /// Flusso compresso con un solo elemento: potrebbe contenere un tar.
    pub fn may_be_compressed_tar(&self) -> bool {
        STREAM_FORMATS.contains(&self.format.to_lowercase().as_str()) && self.entries.len() == 1
    }
}

pub fn parse(output: &str) -> Listing {
    let mut listing = Listing::default();
    let mut lines = output.lines().peekable();

    // Intestazione dell'archivio, tra "--" e "----------".
    let mut in_header = false;
    let mut header_level = 0;
    for line in lines.by_ref() {
        let t = line.trim_end();
        if t == "----------" {
            break;
        }
        if t == "--" {
            in_header = true;
            header_level += 1;
            continue;
        }
        if !in_header {
            continue;
        }
        if let Some((k, v)) = split_kv(t) {
            // Con archivi annidati (es. .exe SFX) l'ultimo blocco è il formato reale.
            match k {
                "Type" => listing.format = v.to_string(),
                "Physical Size" => listing.physical_size = v.parse().unwrap_or(0),
                "Headers Encrypted" | "Encrypted" if header_level > 0 && v == "+" => {
                    listing.headers_encrypted = true;
                }
                _ => {}
            }
        }
    }

    let mut current: Vec<(String, String)> = Vec::new();
    for line in lines.chain(std::iter::once("")) {
        let t = line.trim_end();
        if t.is_empty() {
            if !current.is_empty() {
                if let Some(e) = to_entry(&current) {
                    listing.entries.push(e);
                }
                current.clear();
            }
            continue;
        }
        if let Some((k, v)) = split_kv(t) {
            current.push((k.to_string(), v.to_string()));
        }
    }
    listing
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(" = ").map(|i| (i, 3)).or_else(|| {
        // Valore vuoto: "Symbolic Link = " diventa "Symbolic Link =" dopo il trim.
        line.strip_suffix(" =").map(|s| (s.len(), 2))
    })?;
    let (i, sep) = idx;
    Some((&line[..i], line.get(i + sep..).unwrap_or("")))
}

fn to_entry(kv: &[(String, String)]) -> Option<Entry> {
    let get = |key: &str| kv.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    // I formati a flusso (xz, gzip...) possono non riportare il nome.
    let path = get("Path").unwrap_or("").to_string();
    if kv.is_empty() {
        return None;
    }
    let attrs = get("Attributes").unwrap_or("");
    let mode = get("Mode").unwrap_or("");
    let is_dir = get("Folder") == Some("+") || attrs.starts_with('D') || mode.starts_with('d');
    let symlink = get("Symbolic Link")
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let hardlink = get("Hard Link")
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let unix_link_attr = attrs
        .split_whitespace()
        .any(|tok| tok.len() == 10 && tok.starts_with('l'))
        || mode.starts_with('l');
    Some(Entry {
        path,
        size: get("Size").and_then(|v| v.parse().ok()).unwrap_or(0),
        packed: get("Packed Size").and_then(|v| v.parse().ok()).unwrap_or(0),
        modified: get("Modified").unwrap_or("").to_string(),
        is_dir,
        encrypted: get("Encrypted") == Some("+"),
        link_unknown_target: unix_link_attr && symlink.is_none(),
        symlink,
        hardlink,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
7-Zip (z) 26.05 (arm64) : Copyright (c) 1999-2026 Igor Pavlov : 2026-10-21

Listing archive: a.7z

--
Path = a.7z
Type = 7z
Physical Size = 154
Headers Size = 138
Method = LZMA2:12 7zAES:19
Solid = -
Blocks = 1

----------
Path = src
Size = 0
Packed Size = 0
Modified = 2026-10-25 20:30:00
Attributes = D drwxr-xr-x
Encrypted = -

Path = src/a.txt
Size = 6
Packed Size = 16
Modified = 2026-10-25 20:30:00
Attributes = A -rw-r--r--
CRC = 363A3020
Encrypted = +
Method = LZMA2:12 7zAES:19
Block = 0

Path = src/link
Size = 4
Packed Size = 0
Attributes = A lrwxr-xr-x
Encrypted = -
";

    #[test]
    fn parses_7z_listing() {
        let l = parse(SAMPLE);
        assert_eq!(l.format, "7z");
        assert_eq!(l.physical_size, 154);
        assert_eq!(l.entries.len(), 3);
        assert!(l.entries[0].is_dir);
        assert_eq!(l.entries[1].path, "src/a.txt");
        assert_eq!(l.entries[1].size, 6);
        assert!(l.entries[1].encrypted);
        assert!(l.any_encrypted());
        assert!(l.entries[2].link_unknown_target);
        assert_eq!(l.total_size(), 10);
    }

    #[test]
    fn parses_tar_links_and_compressed_tar() {
        let tar = "--\nPath = x.tar\nType = tar\n\n----------\nPath = lnk\nFolder = -\nSize = 4\nMode = lrw-r--r--\nSymbolic Link = /etc\nHard Link = \n\nPath = d\nFolder = +\nMode = drwxr-xr-x\n";
        let l = parse(tar);
        assert_eq!(l.entries[0].symlink.as_deref(), Some("/etc"));
        assert_eq!(l.entries[0].hardlink, None);
        assert!(!l.entries[0].link_unknown_target);
        assert!(l.entries[1].is_dir);

        let gz = "--\nPath = out.tar.gz\nType = gzip\n\n----------\nPath = arch.tar\nSize = 300\n";
        assert!(parse(gz).may_be_compressed_tar());
        let xz = "--\nPath = x.tar.xz\nType = xz\n\n----------\nSize = 3072\nPacked Size = 168\n";
        assert!(parse(xz).may_be_compressed_tar());
    }
}
