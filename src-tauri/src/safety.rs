//! Protezioni contro archivi malevoli: Zip Slip, link pericolosi,
//! decompression bomb, sovrascritture.

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::listing::Listing;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Codice stabile: absolute-path, path-traversal, unsafe-symlink,
    /// unsafe-hardlink, suspicious-ratio.
    pub kind: String,
    pub path: String,
    pub detail: String,
}

impl Issue {
    fn new(kind: &str, path: &str, detail: impl Into<String>) -> Self {
        Issue {
            kind: kind.into(),
            path: path.into(),
            detail: detail.into(),
        }
    }
}

/// Limiti contro le decompression bomb.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Dimensione massima estraibile in assoluto.
    pub max_total: u64,
    /// Rapporto di compressione oltre il quale l'archivio è sospetto
    /// (si applica solo sopra `ratio_min_size`).
    pub max_ratio: u64,
    pub ratio_min_size: u64,
    /// Spazio libero da lasciare sempre sul disco.
    pub keep_free: u64,
}

pub const GIB: u64 = 1024 * 1024 * 1024;
pub const MIB: u64 = 1024 * 1024;

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_total: 64 * GIB,
            max_ratio: 1000,
            ratio_min_size: GIB,
            keep_free: 512 * MIB,
        }
    }
}

fn parts(p: &str) -> Vec<&str> {
    p.split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != ".")
        .collect()
}

pub fn is_absolute_like(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/')
        || p.starts_with('\\')
        || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
}

/// `true` se il percorso, relativo alla radice, ne esce con `..`.
pub fn escapes(base_parts: &[&str], target: &str) -> bool {
    let mut depth = base_parts.len() as i64;
    for c in parts(target) {
        if c == ".." {
            depth -= 1;
            if depth < 0 {
                return true;
            }
        } else {
            depth += 1;
        }
    }
    false
}

/// Analizza l'elenco prima di estrarre. Non tocca il disco.
pub fn scan_listing(listing: &Listing, limits: &Limits) -> Vec<Issue> {
    let mut issues = Vec::new();
    for e in &listing.entries {
        let p = e.path.as_str();
        if is_absolute_like(p) {
            issues.push(Issue::new("absolute-path", p, "percorso assoluto"));
        } else if parts(p).contains(&"..") {
            issues.push(Issue::new("path-traversal", p, "contiene '..'"));
        }
        let entry_parts = parts(p);
        let parent = &entry_parts[..entry_parts.len().saturating_sub(1)];
        if let Some(t) = &e.symlink {
            if is_absolute_like(t) || escapes(parent, t) {
                issues.push(Issue::new("unsafe-symlink", p, format!("→ {t}")));
            }
        }
        if let Some(t) = &e.hardlink {
            if is_absolute_like(t) || escapes(&[], t) {
                issues.push(Issue::new("unsafe-hardlink", p, format!("→ {t}")));
            }
        }
    }
    let total = listing.total_size();
    if listing.physical_size > 0
        && total >= limits.ratio_min_size
        && total / listing.physical_size.max(1) > limits.max_ratio
    {
        issues.push(Issue::new(
            "suspicious-ratio",
            "",
            format!("{} → {} byte", listing.physical_size, total),
        ));
    }
    issues
}

/// Esito del controllo eseguito dopo l'estrazione nella cartella temporanea.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostScan {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    /// Elementi neutralizzati (link rimossi, permessi ridotti, hard link separati).
    pub fixed: Vec<Issue>,
}

/// Verifica l'albero estratto senza seguire i link:
/// - rimuove i link simbolici che puntano fuori dalla radice;
/// - toglie i bit setuid/setgid/sticky;
/// - separa gli hard link condivisi con file esterni alla radice.
pub fn post_scan(root: &Path) -> std::io::Result<PostScan> {
    let mut res = PostScan::default();
    let canon_root = fs::canonicalize(root)?;
    #[cfg(unix)]
    let mut inodes: HashMap<(u64, u64), (u64, Vec<PathBuf>)> = HashMap::new();
    #[cfg(not(unix))]
    let _ = HashMap::<(), ()>::new();

    for item in WalkDir::new(root).follow_links(false).min_depth(1) {
        let item = item.map_err(std::io::Error::other)?;
        let path = item.path();
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().to_string();
        let ft = item.file_type();
        if ft.is_symlink() {
            let target = fs::read_link(path)?;
            let t = target.to_string_lossy().to_string();
            let rel_parts: Vec<String> = rel
                .parent()
                .map(|p| p.components().filter_map(comp_str).collect())
                .unwrap_or_default();
            let rel_refs: Vec<&str> = rel_parts.iter().map(String::as_str).collect();
            let mut bad = target.is_absolute() || is_absolute_like(&t) || escapes(&rel_refs, &t);
            if !bad {
                if let Ok(real) = fs::canonicalize(path) {
                    bad = !real.starts_with(&canon_root);
                }
            }
            if bad {
                fs::remove_file(path)?;
                res.fixed.push(Issue::new(
                    "unsafe-symlink",
                    &rel_str,
                    format!("rimosso (→ {t})"),
                ));
            }
            continue;
        }
        if ft.is_dir() {
            res.dirs += 1;
            continue;
        }
        if ft.is_file() {
            let meta = item.metadata().map_err(std::io::Error::other)?;
            res.files += 1;
            res.bytes += meta.len();
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                let mode = meta.mode();
                if mode & 0o7000 != 0 {
                    fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))?;
                    res.fixed.push(Issue::new(
                        "special-bits",
                        &rel_str,
                        "setuid/setgid rimossi",
                    ));
                }
                if meta.nlink() > 1 {
                    let e = inodes
                        .entry((meta.dev(), meta.ino()))
                        .or_insert((meta.nlink(), Vec::new()));
                    e.1.push(path.to_path_buf());
                }
            }
        }
    }

    #[cfg(unix)]
    for (_, (nlink, paths)) in inodes {
        if (paths.len() as u64) < nlink {
            // Almeno un collegamento è fuori dalla radice: si sostituisce con una copia.
            for p in paths {
                break_hardlink(&p)?;
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string();
                res.fixed.push(Issue::new(
                    "unsafe-hardlink",
                    &rel,
                    "separato in una copia indipendente",
                ));
            }
        }
    }
    Ok(res)
}

#[cfg(unix)]
fn break_hardlink(p: &Path) -> std::io::Result<()> {
    let dir = p.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    fs::copy(p, tmp.path())?;
    tmp.persist(p).map_err(|e| e.error)?;
    Ok(())
}

fn comp_str(c: Component) -> Option<String> {
    match c {
        Component::Normal(s) => Some(s.to_string_lossy().to_string()),
        _ => None,
    }
}

/// Dimensione attuale di un albero (senza seguire i link), per il watchdog.
pub fn tree_size(root: &Path) -> u64 {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

pub fn available_space(path: &Path) -> Option<u64> {
    fs4::available_space(path).ok()
}

/// Restituisce un percorso libero in `dir`: "nome", "nome (2)", ...
/// Non sovrascrive mai un elemento esistente.
pub fn unique_path(dir: &Path, name: &str, is_dir: bool) -> PathBuf {
    let (stem, ext) = if is_dir {
        (name.to_string(), String::new())
    } else {
        split_name(name)
    };
    let mut candidate = dir.join(name);
    let mut n = 2;
    while fs::symlink_metadata(&candidate).is_ok() {
        candidate = dir.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    candidate
}

/// Divide il nome in base ed estensione, riconoscendo le estensioni doppie.
pub fn split_name(name: &str) -> (String, String) {
    let lower = name.to_lowercase();
    for multi in [
        ".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tar.lz", ".tar.z",
    ] {
        if lower.ends_with(multi) && name.len() > multi.len() {
            let i = name.len() - multi.len();
            return (name[..i].to_string(), name[i..].to_string());
        }
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// Nome della cartella di destinazione ricavato dal nome dell'archivio.
pub fn archive_stem(file_name: &str) -> String {
    let mut name = file_name.to_string();
    // Volumi: archivio.7z.001, archivio.zip.001
    if let Some(i) = name.rfind('.') {
        if name[i + 1..].len() == 3 && name[i + 1..].chars().all(|c| c.is_ascii_digit()) {
            name.truncate(i);
        }
    }
    let (mut stem, _) = split_name(&name);
    let lower = stem.to_lowercase();
    if let Some(i) = lower.rfind(".part") {
        if lower[i + 5..].chars().all(|c| c.is_ascii_digit()) && i + 5 < lower.len() {
            stem.truncate(i);
        }
    }
    // Niente cartelle nascoste: ".zip" diventa "archivio".
    let stem = stem.trim().trim_start_matches('.').to_string();
    let only_extension = !name.trim_start_matches('.').contains('.') && name.starts_with('.');
    if stem.is_empty() || only_extension {
        "archivio".into()
    } else {
        stem
    }
}

/// Valida un nome di file scelto dall'utente (niente separatori o nomi speciali).
pub fn valid_file_name(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty()
        && n != "."
        && n != ".."
        && !n.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'])
        && n.len() <= 255
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listing::Entry;

    fn entry(path: &str) -> Entry {
        Entry {
            path: path.into(),
            size: 1,
            ..Default::default()
        }
    }

    #[test]
    fn detects_traversal_and_absolute() {
        let l = Listing {
            entries: vec![
                entry("../../evil.txt"),
                entry("/etc/passwd"),
                entry("C:\\Windows\\x.dll"),
                entry("ok/../../x"),
                entry("ok/fine.txt"),
                entry("ok/..hidden"),
            ],
            ..Default::default()
        };
        let kinds: Vec<_> = scan_listing(&l, &Limits::default())
            .into_iter()
            .map(|i| i.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                "path-traversal",
                "absolute-path",
                "absolute-path",
                "path-traversal"
            ]
        );
    }

    #[test]
    fn detects_dangerous_links() {
        let mut a = entry("lnk");
        a.symlink = Some("/etc".into());
        let mut b = entry("d/rel");
        b.symlink = Some("../../outside".into());
        let mut c = entry("d/ok");
        c.symlink = Some("../sibling".into());
        let mut d = entry("h");
        d.hardlink = Some("../x".into());
        let l = Listing {
            entries: vec![a, b, c, d],
            ..Default::default()
        };
        let kinds: Vec<_> = scan_listing(&l, &Limits::default())
            .into_iter()
            .map(|i| i.path)
            .collect();
        assert_eq!(kinds, vec!["lnk", "d/rel", "h"]);
    }

    #[test]
    fn detects_bomb_ratio() {
        let mut e = entry("big");
        e.size = 50 * GIB;
        let l = Listing {
            physical_size: 10 * MIB,
            entries: vec![e],
            ..Default::default()
        };
        assert_eq!(
            scan_listing(&l, &Limits::default())[0].kind,
            "suspicious-ratio"
        );
    }

    #[test]
    fn names() {
        assert_eq!(archive_stem("foto.zip"), "foto");
        assert_eq!(archive_stem("backup.tar.gz"), "backup");
        assert_eq!(archive_stem("dati.7z.001"), "dati");
        assert_eq!(archive_stem("film.part1.rar"), "film");
        assert_eq!(archive_stem(".zip"), "archivio");
        assert_eq!(archive_stem(".7z"), "archivio");
        assert_eq!(archive_stem(".config.zip"), "config");
        assert_eq!(split_name("a.b.txt"), ("a.b".into(), ".txt".into()));
        assert!(valid_file_name("Archivio 2026.zip"));
        assert!(!valid_file_name("../x.zip"));
        assert!(!valid_file_name(""));
    }

    #[test]
    fn unique_never_overwrites() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.zip"), b"x").unwrap();
        fs::write(d.path().join("a (2).zip"), b"x").unwrap();
        assert_eq!(
            unique_path(d.path(), "a.zip", false),
            d.path().join("a (3).zip")
        );
        assert_eq!(
            unique_path(d.path(), "b.zip", false),
            d.path().join("b.zip")
        );
        fs::create_dir(d.path().join("cartella")).unwrap();
        assert_eq!(
            unique_path(d.path(), "cartella", true),
            d.path().join("cartella (2)")
        );
    }

    #[cfg(unix)]
    #[test]
    fn post_scan_neutralizes() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let outside = tempfile::tempdir().unwrap();
        let ext_file = outside.path().join("segreto.txt");
        fs::write(&ext_file, b"originale").unwrap();

        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir(r.join("sub")).unwrap();
        fs::write(r.join("sub/a.txt"), b"ciao").unwrap();
        symlink("/etc", r.join("abs")).unwrap();
        symlink("../../fuori", r.join("sub/rel")).unwrap();
        symlink("a.txt", r.join("sub/buono")).unwrap();
        fs::write(r.join("suid"), b"x").unwrap();
        fs::set_permissions(r.join("suid"), fs::Permissions::from_mode(0o4755)).unwrap();
        fs::hard_link(&ext_file, r.join("hl.txt")).unwrap();

        let res = post_scan(r).unwrap();
        assert!(fs::symlink_metadata(r.join("abs")).is_err());
        assert!(fs::symlink_metadata(r.join("sub/rel")).is_err());
        assert!(fs::symlink_metadata(r.join("sub/buono")).is_ok());
        let mode = fs::metadata(r.join("suid")).unwrap().permissions().mode();
        assert_eq!(mode & 0o7000, 0);
        // L'hard link è stato separato: modificarlo non tocca il file esterno.
        fs::write(r.join("hl.txt"), b"modificato").unwrap();
        assert_eq!(fs::read(&ext_file).unwrap(), b"originale");
        let kinds: Vec<_> = res.fixed.iter().map(|i| i.kind.as_str()).collect();
        assert!(kinds.contains(&"unsafe-symlink"));
        assert!(kinds.contains(&"special-bits"));
        assert!(kinds.contains(&"unsafe-hardlink"));
        assert_eq!(res.files, 3);
    }
}
