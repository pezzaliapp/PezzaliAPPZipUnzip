//! Localizzazione ed esecuzione del motore 7-Zip ufficiale (CLI).
//!
//! L'app non incorpora codice di 7-Zip: lo esegue come processo separato,
//! senza shell, passando gli argomenti come vettore. Le password non compaiono
//! mai sulla riga di comando: vengono scritte su stdin quando 7-Zip le chiede.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::error::OpError;

#[cfg(windows)]
const EXE_NAMES: &[&str] = &["7z.exe", "7zz.exe", "7za.exe"];
#[cfg(not(windows))]
const EXE_NAMES: &[&str] = &["7zz", "7z", "7za"];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    pub path: PathBuf,
    pub version: String,
    pub bundled: bool,
}

impl Engine {
    /// Cerca 7-Zip: prima nella cartella `engine` inclusa nel pacchetto,
    /// poi nel PATH, poi nei percorsi di installazione abituali.
    pub fn locate(bundled_dir: Option<&Path>) -> Option<Engine> {
        if let Some(dir) = bundled_dir {
            for name in EXE_NAMES {
                let p = dir.join(name);
                if p.is_file() {
                    if let Some(e) = Self::probe(&p, true) {
                        return Some(e);
                    }
                }
            }
        }
        for p in system_candidates() {
            if p.is_file() {
                if let Some(e) = Self::probe(&p, false) {
                    return Some(e);
                }
            }
        }
        None
    }

    /// Verifica che l'eseguibile sia davvero 7-Zip leggendone il banner.
    pub fn probe(path: &Path, bundled: bool) -> Option<Engine> {
        let out = command(path).arg("i").stdin(Stdio::null()).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        parse_banner(&text).map(|version| Engine {
            path: path.to_path_buf(),
            version,
            bundled,
        })
    }
}

/// Estrae la versione dal banner, es. "7-Zip (z) 26.05 (arm64) : Copyright ...".
pub fn parse_banner(text: &str) -> Option<String> {
    let line = text.lines().find(|l| l.trim_start().starts_with("7-Zip"))?;
    line.split_whitespace()
        .find(|tok| {
            let mut parts = tok.split('.');
            matches!((parts.next(), parts.next()), (Some(a), Some(b))
                if !a.is_empty() && a.chars().all(|c| c.is_ascii_digit())
                    && !b.is_empty() && b.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_string)
}

fn system_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for name in EXE_NAMES {
                out.push(dir.join(name));
            }
        }
    }
    #[cfg(windows)]
    {
        for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(pf) = std::env::var_os(var) {
                out.push(PathBuf::from(pf).join("7-Zip").join("7z.exe"));
            }
        }
    }
    #[cfg(not(windows))]
    {
        // Le app grafiche su macOS/Linux spesso partono con un PATH ridotto.
        for dir in [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/snap/bin",
        ] {
            for name in EXE_NAMES {
                out.push(Path::new(dir).join(name));
            }
        }
    }
    out
}

/// Crea un `Command` senza finestra di console su Windows.
pub fn command(path: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Origine dello stdin di uno stadio della pipeline.
pub enum Input {
    /// Nessun input: 7-Zip riceve EOF se chiede una password.
    Closed,
    /// Testo scritto su stdin (password, eventualmente ripetuta per conferma).
    Text(String),
    /// Collegato allo stdout dello stadio precedente.
    Previous,
}

pub struct Stage {
    pub args: Vec<OsString>,
    pub input: Input,
    /// Se `true` lo stdout è dati (pipeline) e non va letto come testo.
    pub stdout_is_data: bool,
}

impl Stage {
    pub fn new(args: Vec<OsString>, input: Input) -> Self {
        Stage {
            args,
            input,
            stdout_is_data: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub codes: Vec<i32>,
}

/// Controllo periodico durante l'esecuzione: restituisce un errore per
/// interrompere il lavoro (es. superamento del limite di dimensione).
pub type Watchdog<'a> = &'a mut dyn FnMut() -> Result<(), OpError>;

/// Esegue uno o due stadi di 7-Zip collegati in pipeline.
/// La percentuale di avanzamento viene letta dallo stdout o dallo stderr
/// di qualsiasi stadio (7-Zip usa `\b` per riscrivere la riga).
pub fn run(
    engine: &Path,
    stages: Vec<Stage>,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(u8) + Sync),
    mut watchdog: Option<Watchdog>,
) -> Result<RunOutput, OpError> {
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;
    let mut writers = Vec::new();

    for stage in &stages {
        let mut cmd = command(engine);
        cmd.args(&stage.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        match &stage.input {
            Input::Closed => {
                cmd.stdin(Stdio::null());
            }
            Input::Text(_) => {
                cmd.stdin(Stdio::piped());
            }
            Input::Previous => {
                let out = prev_stdout
                    .take()
                    .ok_or_else(|| OpError::engine("pipeline non valida"))?;
                cmd.stdin(Stdio::from(out));
            }
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                kill_all(&mut children);
                return Err(OpError::engine(format!("avvio di 7-Zip non riuscito: {e}")));
            }
        };
        if let Input::Text(text) = &stage.input {
            if let Some(mut stdin) = child.stdin.take() {
                let text = text.clone();
                // Scrittura in un thread: 7-Zip legge stdin solo quando serve.
                writers.push(thread::spawn(move || {
                    let _ = stdin.write_all(text.as_bytes());
                }));
            }
        }
        if stage.stdout_is_data {
            prev_stdout = child.stdout.take();
        }
        children.push(child);
    }

    let collected: Arc<Mutex<(Vec<u8>, Vec<u8>)>> = Arc::default();
    thread::scope(|scope| -> Result<(), OpError> {
        let mut readers = Vec::new();
        for child in children.iter_mut() {
            let streams: Vec<(Box<dyn Read + Send>, bool)> = [
                child
                    .stdout
                    .take()
                    .map(|s| (Box::new(s) as Box<dyn Read + Send>, true)),
                child
                    .stderr
                    .take()
                    .map(|s| (Box::new(s) as Box<dyn Read + Send>, false)),
            ]
            .into_iter()
            .flatten()
            .collect();
            for (mut stream, is_stdout) in streams {
                let collected = Arc::clone(&collected);
                readers.push(scope.spawn(move || {
                    let mut buf = [0u8; 8192];
                    let mut tail = String::new();
                    loop {
                        let n = match stream.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => n,
                        };
                        let chunk = &buf[..n];
                        tail.push_str(&String::from_utf8_lossy(chunk));
                        if let Some(p) = last_percent(&tail) {
                            on_progress(p);
                        }
                        if tail.len() > 256 {
                            let cut = tail.len() - 64;
                            let cut = (cut..tail.len())
                                .find(|i| tail.is_char_boundary(*i))
                                .unwrap_or(0);
                            tail.drain(..cut);
                        }
                        let mut c = collected.lock().unwrap();
                        let target = if is_stdout { &mut c.0 } else { &mut c.1 };
                        if target.len() < 4 * 1024 * 1024 {
                            target.extend_from_slice(chunk);
                        }
                    }
                }));
            }
        }

        // Ciclo di supervisione: annullamento, watchdog, terminazione.
        let mut abort: Option<OpError> = None;
        loop {
            if cancel.load(Ordering::SeqCst) {
                abort = Some(OpError::cancelled());
            } else if let Some(w) = watchdog.as_mut() {
                if let Err(e) = w() {
                    abort = Some(e);
                }
            }
            if abort.is_some() {
                kill_all(&mut children);
                break;
            }
            let all_done = children
                .iter_mut()
                .all(|c| matches!(c.try_wait(), Ok(Some(_))));
            if all_done {
                break;
            }
            thread::sleep(Duration::from_millis(120));
        }
        for r in readers.drain(..) {
            let _ = r.join();
        }
        match abort {
            Some(e) => Err(e),
            None => Ok(()),
        }
    })?;
    for w in writers {
        let _ = w.join();
    }

    let mut codes = Vec::new();
    for c in children.iter_mut() {
        let code = c.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
        codes.push(code);
    }
    let c = collected.lock().unwrap();
    Ok(RunOutput {
        stdout: String::from_utf8_lossy(&c.0).replace('\r', ""),
        stderr: clean_text(&c.1),
        codes,
    })
}

fn kill_all(children: &mut [Child]) {
    for c in children.iter_mut() {
        let _ = c.kill();
        let _ = c.wait();
    }
}

/// Rimuove i caratteri di controllo usati da 7-Zip per l'animazione.
fn clean_text(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    s.chars()
        .map(|c| if c == '\u{8}' || c == '\r' { '\n' } else { c })
        .collect::<String>()
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ultima percentuale "NN%" presente nel testo.
pub fn last_percent(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    let mut found = None;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'%' {
            let mut j = i;
            while j > 0 && bytes[j - 1].is_ascii_digit() {
                j -= 1;
            }
            if j < i && i - j <= 3 {
                if let Ok(v) = text[j..i].parse::<u8>() {
                    if v <= 100 {
                        found = Some(v);
                    }
                }
            }
        }
    }
    found
}

/// Converte l'esito di 7-Zip in un errore comprensibile.
pub fn check(out: &RunOutput, had_password: bool) -> Result<(), OpError> {
    if out.codes.iter().all(|c| *c == 0) {
        return Ok(());
    }
    Err(classify(&out.stderr, &out.stdout, had_password))
}

pub fn classify(stderr: &str, stdout: &str, had_password: bool) -> OpError {
    let all = format!("{stderr}\n{stdout}").replace('\u{8}', " ");
    let low = all.to_lowercase();
    let first_error = error_summary(stderr).unwrap_or_else(|| {
        all.lines()
            .map(str::trim)
            .rfind(|l| !l.is_empty())
            .unwrap_or("")
            .to_string()
    });
    if low.contains("wrong password")
        || low.contains("can not open encrypted archive")
        || low.contains("cannot open encrypted archive")
        || (low.contains("break signaled") && !had_password)
    {
        return if had_password {
            OpError::new("wrong-password", "Password errata o archivio danneggiato.")
        } else {
            OpError::new("need-password", "L'archivio è protetto da password.")
        };
    }
    if low.contains("no space left")
        || low.contains("there is not enough space")
        || low.contains("disk full")
    {
        return OpError::new("no-space", "Spazio su disco insufficiente.");
    }
    if low.contains("is not archive")
        || low.contains("cannot open the file as archive")
        || low.contains("can not open the file as archive")
    {
        return OpError::new(
            "not-archive",
            "Il file non è un archivio riconosciuto o è danneggiato.",
        );
    }
    if low.contains("data error")
        || low.contains("crc failed")
        || low.contains("unexpected end")
        || low.contains("headers error")
    {
        return OpError::with_details(
            "corrupt",
            "L'archivio è danneggiato o incompleto.",
            vec![first_error],
        );
    }
    OpError::with_details("engine", "7-Zip ha segnalato un errore.", vec![first_error])
}

/// Righe di errore di 7-Zip, unendo "System ERROR:" alla riga successiva.
fn error_summary(stderr: &str) -> Option<String> {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.contains("ERROR") || l.starts_with("Error") {
            if l.ends_with(':') {
                out.push(format!("{l} {}", lines.get(i + 1).unwrap_or(&"")));
            } else {
                out.push(l.to_string());
            }
        }
        if out.len() == 3 {
            break;
        }
    }
    (!out.is_empty()).then(|| out.join(" | "))
}

/// Concatena un prefisso di opzione e un percorso, es. `-o` + `/dest`.
pub fn switch_path(prefix: &str, path: &Path) -> OsString {
    let mut s = OsString::from(prefix);
    s.push(path.as_os_str());
    s
}

pub fn os<S: AsRef<OsStr>>(s: S) -> OsString {
    s.as_ref().to_os_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_version() {
        let b = "\n7-Zip (z) 26.05 (arm64) : Copyright (c) 1999-2026 Igor Pavlov : 2026-10-21\n";
        assert_eq!(parse_banner(b).as_deref(), Some("26.05"));
        let w = "7-Zip 24.09 (x64) : Copyright (c) 1999-2024 Igor Pavlov : 2024-11-29";
        assert_eq!(parse_banner(w).as_deref(), Some("24.09"));
        assert_eq!(parse_banner("bash: command not found"), None);
    }

    #[test]
    fn percent_parsing() {
        assert_eq!(last_percent("  0%\u{8}\u{8}\u{8}  8% 1 + big"), Some(8));
        assert_eq!(last_percent("17% 2 + x\u{8}\u{8}100%"), Some(100));
        assert_eq!(last_percent("nessuna"), None);
        assert_eq!(last_percent("1234%"), None);
    }

    #[test]
    fn classify_errors() {
        assert_eq!(
            classify("ERROR: Wrong password : a.txt", "", true).code,
            "wrong-password"
        );
        assert_eq!(
            classify("ERROR: Wrong password : a.txt", "", false).code,
            "need-password"
        );
        assert_eq!(
            classify("Can not open the file as archive", "", false).code,
            "not-archive"
        );
        assert_eq!(classify("ERROR: CRC Failed : a", "", false).code, "corrupt");
        assert_eq!(classify("something odd", "", false).code, "engine");
        let e = classify("\nSystem ERROR:\nE_INVALIDARG : bad\n", "  0M Scan", false);
        assert_eq!(e.details, vec!["System ERROR: E_INVALIDARG : bad"]);
    }
}
