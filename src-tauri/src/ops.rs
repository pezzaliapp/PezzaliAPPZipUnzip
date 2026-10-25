//! Operazioni sugli archivi, indipendenti dall'interfaccia (testabili da sole).
//!
//! Regole comuni:
//! - si lavora sempre in una cartella temporanea nella destinazione e solo
//!   alla fine si sposta il risultato con un nome libero: nessuna sovrascrittura;
//! - i file originali non vengono mai modificati né eliminati;
//! - i contenuti estratti non vengono mai eseguiti.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::engine::{self, os, switch_path, Input, Stage};
use crate::error::OpError;
use crate::listing::{self, Listing};
use crate::safety::{self, Issue, Limits, MIB};

const TMP_PREFIX: &str = ".pezzaliapp-";

pub struct Ctx<'a> {
    pub engine: &'a Path,
    pub cancel: &'a AtomicBool,
    /// (fase, percentuale)
    pub progress: &'a (dyn Fn(&str, u8) + Sync),
    pub limits: Limits,
}

fn password_input(pw: Option<&str>, repeat: bool) -> Input {
    match pw.filter(|p| !p.is_empty()) {
        Some(p) if repeat => Input::Text(format!("{p}\n{p}\n")),
        Some(p) => Input::Text(format!("{p}\n")),
        None => Input::Closed,
    }
}

fn has_pw(pw: Option<&str>) -> bool {
    pw.is_some_and(|p| !p.is_empty())
}

fn require_file(p: &Path) -> Result<(), OpError> {
    match fs::metadata(p) {
        Ok(m) if m.is_file() => Ok(()),
        Ok(_) => Err(OpError::new(
            "not-found",
            "Il percorso indicato non è un file.",
        )),
        Err(_) => Err(OpError::new("not-found", "File non trovato.")),
    }
}

fn require_dir(p: &Path) -> Result<(), OpError> {
    match fs::metadata(p) {
        Ok(m) if m.is_dir() => Ok(()),
        _ => Err(OpError::new(
            "not-found",
            "Cartella di destinazione non trovata.",
        )),
    }
}

fn tar_ext(format: &str) -> &str {
    match format.to_lowercase().as_str() {
        "gzip" => "gz",
        "bzip2" => "bz2",
        "zstd" => "zst",
        "xz" => "xz",
        "lzma" => "lzma",
        _ => "?",
    }
}

// ---------------------------------------------------------------- elenco

pub fn list(ctx: &Ctx, archive: &Path, password: Option<&str>) -> Result<Listing, OpError> {
    require_file(archive)?;
    let args = vec![os("l"), os("-slt"), os("-sccUTF-8"), os("--"), os(archive)];
    let out = engine::run(
        ctx.engine,
        vec![Stage::new(args, password_input(password, false))],
        ctx.cancel,
        &|_| {},
        None,
    )?;
    engine::check(&out, has_pw(password))?;
    let mut l = listing::parse(&out.stdout);
    if l.format.is_empty() {
        return Err(OpError::new(
            "not-archive",
            "Il file non è un archivio riconosciuto.",
        ));
    }
    if l.may_be_compressed_tar() {
        // Se il contenuto non è un tar si resta sull'elenco esterno
        // (es. un singolo file .gz).
        if let Ok(mut inner) = list_compressed_tar(ctx, archive) {
            if !inner.entries.is_empty() && inner.format.eq_ignore_ascii_case("tar") {
                inner.format = format!("tar.{}", tar_ext(&l.format));
                inner.physical_size = l.physical_size;
                l = inner;
            }
        }
    }
    Ok(l)
}

fn list_compressed_tar(ctx: &Ctx, archive: &Path) -> Result<Listing, OpError> {
    let mut s1 = Stage::new(
        vec![
            os("x"),
            os("-so"),
            os("-bsp0"),
            os("-sccUTF-8"),
            os("--"),
            os(archive),
        ],
        Input::Closed,
    );
    s1.stdout_is_data = true;
    let s2 = Stage::new(
        vec![os("l"), os("-slt"), os("-si"), os("-ttar"), os("-sccUTF-8")],
        Input::Previous,
    );
    let out = engine::run(ctx.engine, vec![s1, s2], ctx.cancel, &|_| {}, None)?;
    engine::check(&out, false)?;
    Ok(listing::parse(&out.stdout))
}

// ---------------------------------------------------------------- estrazione

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResult {
    pub output: PathBuf,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    /// Problemi accettati dall'utente e correzioni applicate.
    pub warnings: Vec<Issue>,
}

pub fn extract(
    ctx: &Ctx,
    archive: &Path,
    dest_dir: &Path,
    password: Option<&str>,
    allow_unsafe: bool,
) -> Result<ExtractResult, OpError> {
    require_file(archive)?;
    require_dir(dest_dir)?;
    (ctx.progress)("analyze", 0);
    let listing = list(ctx, archive, password)?;

    let issues = safety::scan_listing(&listing, &ctx.limits);
    if !issues.is_empty() && !allow_unsafe {
        let mut e = OpError::new(
            "unsafe",
            "L'archivio contiene elementi potenzialmente pericolosi.",
        );
        e.issues = issues;
        return Err(e);
    }
    if listing.any_encrypted() && !has_pw(password) {
        return Err(OpError::new(
            "need-password",
            "L'archivio è protetto da password.",
        ));
    }

    let total = listing.total_size();
    if total > ctx.limits.max_total {
        return Err(OpError::with_details(
            "too-large",
            "L'archivio supera la dimensione massima consentita.",
            vec![format!("{total} byte")],
        ));
    }
    let free = safety::available_space(dest_dir);
    if let Some(free) = free {
        if free < total.saturating_add(ctx.limits.keep_free) {
            return Err(OpError::with_details(
                "no-space",
                "Spazio su disco insufficiente.",
                vec![format!("servono {total} byte, liberi {free}")],
            ));
        }
    }

    let staging = tempfile::Builder::new()
        .prefix(TMP_PREFIX)
        .tempdir_in(dest_dir)
        .map_err(|e| OpError::io("Impossibile creare la cartella temporanea.", e))?;
    let sroot = staging.path().to_path_buf();

    // Limite reale: le dimensioni dichiarate possono essere false.
    let cap = if total > 0 {
        total
            .saturating_add((total / 20).max(64 * MIB))
            .min(ctx.limits.max_total)
    } else {
        ctx.limits.max_total
    };
    let keep_free = ctx.limits.keep_free / 2;
    let mut last = Instant::now();
    let mut interval = Duration::from_millis(250);
    let mut watchdog = || -> Result<(), OpError> {
        if last.elapsed() < interval {
            return Ok(());
        }
        // Intervallo adattivo: misurare l'albero non deve rallentare l'estrazione.
        let t = Instant::now();
        let size = safety::tree_size(&sroot);
        interval = (t.elapsed() * 10).clamp(Duration::from_millis(250), Duration::from_secs(5));
        last = Instant::now();
        if size > cap {
            return Err(OpError::with_details(
                "bomb",
                "Estrazione interrotta: i dati superano la dimensione dichiarata (possibile decompression bomb).",
                vec![format!("{size} byte > limite {cap}")],
            ));
        }
        if let Some(free) = safety::available_space(&sroot) {
            if free < keep_free {
                return Err(OpError::new("no-space", "Spazio su disco insufficiente."));
            }
        }
        Ok(())
    };

    let out_switch = switch_path("-o", &sroot);
    let stages = if listing.format.starts_with("tar.") {
        let mut s1 = Stage::new(
            vec![
                os("x"),
                os("-so"),
                os("-bsp2"),
                os("-sccUTF-8"),
                os("--"),
                os(archive),
            ],
            Input::Closed,
        );
        s1.stdout_is_data = true;
        let s2 = Stage::new(
            vec![
                os("x"),
                os("-si"),
                os("-ttar"),
                os("-aou"),
                os("-bsp0"),
                os("-bso0"),
                os("-sccUTF-8"),
                out_switch,
            ],
            Input::Previous,
        );
        vec![s1, s2]
    } else {
        vec![Stage::new(
            vec![
                os("x"),
                os("-aou"),
                os("-bsp1"),
                os("-bso0"),
                os("-sccUTF-8"),
                out_switch,
                os("--"),
                os(archive),
            ],
            password_input(password, false),
        )]
    };
    let progress = |p: u8| (ctx.progress)("extract", p);
    let out = engine::run(
        ctx.engine,
        stages,
        ctx.cancel,
        &progress,
        Some(&mut watchdog),
    )?;

    let mut warnings: Vec<Issue> = if allow_unsafe { issues } else { Vec::new() };
    if let Err(e) = engine::check(&out, has_pw(password)) {
        // 7-Zip rifiuta da sé i link pericolosi: se sono gli unici errori
        // e l'utente ha accettato il rischio, l'estrazione prosegue.
        let errors: Vec<&str> = out.stderr.lines().filter(|l| l.contains("ERROR")).collect();
        let only_dangerous = !errors.is_empty()
            && errors
                .iter()
                .all(|l| l.contains("Dangerous") || l.contains("Not a directory"));
        if !(allow_unsafe && only_dangerous) {
            return Err(e);
        }
        for l in errors {
            warnings.push(Issue {
                kind: "skipped".into(),
                path: String::new(),
                detail: l.trim_start_matches("ERROR:").trim().to_string(),
            });
        }
    }
    (ctx.progress)("verify", 100);

    let scan = safety::post_scan(&sroot)
        .map_err(|e| OpError::io("Verifica dei file estratti non riuscita.", e))?;
    if scan.bytes > cap {
        return Err(OpError::with_details(
            "bomb",
            "Estrazione interrotta: i dati superano la dimensione dichiarata (possibile decompression bomb).",
            vec![format!("{} byte > limite {cap}", scan.bytes)],
        ));
    }
    warnings.extend(scan.fixed.iter().cloned());
    #[cfg(target_os = "macos")]
    propagate_quarantine(archive, &sroot);

    let output = place(staging, dest_dir, archive)?;
    Ok(ExtractResult {
        output,
        files: scan.files,
        dirs: scan.dirs,
        bytes: scan.bytes,
        warnings,
    })
}

/// Sposta il contenuto estratto nella destinazione finale.
/// Un solo elemento di primo livello viene spostato così com'è,
/// altrimenti si crea una cartella con il nome dell'archivio.
fn place(staging: tempfile::TempDir, dest_dir: &Path, archive: &Path) -> Result<PathBuf, OpError> {
    let sroot = staging.path().to_path_buf();
    let top: Vec<_> = fs::read_dir(&sroot)
        .map_err(|e| OpError::io("Lettura dei file estratti non riuscita.", e))?
        .filter_map(Result::ok)
        .collect();
    let file_name = archive
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    if top.len() == 1 {
        let item = &top[0];
        let name = item.file_name().to_string_lossy().to_string();
        let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let target = safety::unique_path(dest_dir, &name, is_dir);
        fs::rename(item.path(), &target)
            .map_err(|e| OpError::io("Spostamento non riuscito.", e))?;
        drop(staging); // cartella temporanea ormai vuota
        return Ok(target);
    }
    let target = safety::unique_path(dest_dir, &safety::archive_stem(&file_name), true);
    let kept = staging.keep();
    if let Err(e) = fs::rename(&kept, &target) {
        let _ = fs::remove_dir_all(&kept);
        return Err(OpError::io("Spostamento non riuscito.", e));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // La cartella temporanea nasce con permessi 0700.
        let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o755));
    }
    Ok(target)
}

/// Su macOS i file estratti da un archivio scaricato ereditano l'attributo
/// di quarantena, così Gatekeeper continua a proteggerli.
#[cfg(target_os = "macos")]
fn propagate_quarantine(archive: &Path, root: &Path) {
    const Q: &str = "com.apple.quarantine";
    let Ok(Some(value)) = xattr::get(archive, Q) else {
        return;
    };
    for e in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !e.file_type().is_symlink() {
            let _ = xattr::set(e.path(), Q, &value);
        }
    }
}

// ---------------------------------------------------------------- compressione

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    #[serde(rename = "7z")]
    SevenZ,
    Zip,
    Tar,
    TarGz,
    TarXz,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::SevenZ => ".7z",
            Format::Zip => ".zip",
            Format::Tar => ".tar",
            Format::TarGz => ".tar.gz",
            Format::TarXz => ".tar.xz",
        }
    }
    pub fn supports_password(self) -> bool {
        matches!(self, Format::SevenZ | Format::Zip)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    pub format: Format,
    /// 0 = solo archiviazione … 9 = ultra
    pub level: u8,
    pub password: Option<String>,
    /// Solo 7z: cifra anche i nomi dei file.
    pub encrypt_names: bool,
    /// Esclude file di sistema come .DS_Store e Thumbs.db.
    pub exclude_system_files: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressResult {
    pub output: PathBuf,
    pub size: u64,
}

pub fn compress(
    ctx: &Ctx,
    sources: &[PathBuf],
    out_dir: &Path,
    name: &str,
    opts: &CompressOptions,
) -> Result<CompressResult, OpError> {
    if sources.is_empty() {
        return Err(OpError::new("no-input", "Nessun file da comprimere."));
    }
    for s in sources {
        if fs::symlink_metadata(s).is_err() {
            return Err(OpError::with_details(
                "not-found",
                "File non trovato.",
                vec![s.display().to_string()],
            ));
        }
    }
    require_dir(out_dir)?;
    let mut name = name.trim().to_string();
    if !safety::valid_file_name(&name) {
        return Err(OpError::new("bad-name", "Nome dell'archivio non valido."));
    }
    let ext = opts.format.extension();
    if !name.to_lowercase().ends_with(ext) {
        name.push_str(ext);
    }
    let password = opts.password.as_deref().filter(|p| !p.is_empty());
    if password.is_some() && !opts.format.supports_password() {
        return Err(OpError::new(
            "bad-option",
            "Questo formato non supporta la password.",
        ));
    }

    if opts.format == Format::Zip && password.is_some_and(|p| !p.is_ascii()) {
        // 7-Zip accetta solo caratteri ASCII nelle password ZIP.
        return Err(OpError::new(
            "zip-password-ascii",
            "Le password ZIP possono contenere solo caratteri ASCII: usa il formato 7z per lettere accentate o simboli.",
        ));
    }

    let staging = tempfile::Builder::new()
        .prefix(TMP_PREFIX)
        .tempdir_in(out_dir)
        .map_err(|e| OpError::io("Impossibile creare la cartella temporanea.", e))?;
    let tmp_out = staging.path().join(&name);
    let level = opts.level.min(9);

    let mut common: Vec<OsString> = vec![
        os("-snl"),
        os("-sccUTF-8"),
        os(format!("-xr!{TMP_PREFIX}*")),
    ];
    if opts.exclude_system_files {
        for pat in [
            "-xr!.DS_Store",
            "-xr!._*",
            "-xr!Thumbs.db",
            "-xr!desktop.ini",
        ] {
            common.push(os(pat));
        }
    }
    let src_args = sources.iter().map(os).collect::<Vec<_>>();

    let stages = match opts.format {
        Format::SevenZ | Format::Zip | Format::Tar => {
            let mut a = vec![os("a"), os("-bsp1"), os("-bso0")];
            match opts.format {
                Format::SevenZ => {
                    a.push(os("-t7z"));
                    a.push(os(format!("-mx={level}")));
                    if password.is_some() && opts.encrypt_names {
                        a.push(os("-mhe=on"));
                    }
                }
                Format::Zip => {
                    a.push(os("-tzip"));
                    a.push(os(format!("-mx={level}")));
                    if password.is_some() {
                        a.push(os("-mem=AES256"));
                    }
                }
                _ => a.push(os("-ttar")),
            }
            if password.is_some() {
                a.push(os("-p"));
            }
            a.extend(common);
            a.push(os("--"));
            a.push(os(&tmp_out));
            a.extend(src_args);
            vec![Stage::new(a, password_input(password, true))]
        }
        Format::TarGz | Format::TarXz => {
            let mut a = vec![os("a"), os("-ttar"), os("-so"), os("-an"), os("-bsp2")];
            a.extend(common);
            a.push(os("--"));
            a.extend(src_args);
            let mut s1 = Stage::new(a, Input::Closed);
            s1.stdout_is_data = true;
            let inner = safety::split_name(&name).0 + ".tar";
            let t = if opts.format == Format::TarGz {
                "-tgzip"
            } else {
                "-txz"
            };
            let s2 = Stage::new(
                vec![
                    os("a"),
                    os(t),
                    os(format!("-mx={level}")),
                    os(format!("-si{inner}")),
                    os("-bsp0"),
                    os("-bso0"),
                    os("--"),
                    os(&tmp_out),
                ],
                Input::Previous,
            );
            vec![s1, s2]
        }
    };

    let keep_free = 128 * MIB;
    let sroot = staging.path().to_path_buf();
    let mut last = Instant::now();
    let mut watchdog = || -> Result<(), OpError> {
        if last.elapsed() >= Duration::from_secs(1) {
            last = Instant::now();
            if safety::available_space(&sroot).is_some_and(|f| f < keep_free) {
                return Err(OpError::new("no-space", "Spazio su disco insufficiente."));
            }
        }
        Ok(())
    };
    let progress = |p: u8| (ctx.progress)("compress", p);
    let out = engine::run(
        ctx.engine,
        stages,
        ctx.cancel,
        &progress,
        Some(&mut watchdog),
    )?;
    engine::check(&out, password.is_some())?;
    if !tmp_out.is_file() {
        return Err(OpError::engine("7-Zip non ha creato l'archivio."));
    }

    let target = safety::unique_path(out_dir, &name, false);
    fs::rename(&tmp_out, &target).map_err(|e| OpError::io("Spostamento non riuscito.", e))?;
    drop(staging);
    let size = fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    Ok(CompressResult {
        output: target,
        size,
    })
}

// ---------------------------------------------------------------- verifica

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub format: String,
    pub files: usize,
}

pub fn test(ctx: &Ctx, archive: &Path, password: Option<&str>) -> Result<TestResult, OpError> {
    let listing = list(ctx, archive, password)?;
    if listing.any_encrypted() && !has_pw(password) {
        return Err(OpError::new(
            "need-password",
            "L'archivio è protetto da password.",
        ));
    }
    let args = vec![
        os("t"),
        os("-bsp1"),
        os("-bso0"),
        os("-sccUTF-8"),
        os("--"),
        os(archive),
    ];
    let progress = |p: u8| (ctx.progress)("test", p);
    let out = engine::run(
        ctx.engine,
        vec![Stage::new(args, password_input(password, false))],
        ctx.cancel,
        &progress,
        None,
    )?;
    engine::check(&out, has_pw(password))?;
    Ok(TestResult {
        ok: true,
        format: listing.format.clone(),
        files: listing.entries.iter().filter(|e| !e.is_dir).count(),
    })
}
