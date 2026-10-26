//! Test d'integrazione con il vero motore 7-Zip.
//! Il motore viene cercato in `src-tauri/engine/` e poi nel sistema;
//! se manca i test falliscono (nessun esito inventato).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use zipunzip_lib::engine::Engine;
use zipunzip_lib::ops::{self, CompressOptions, Ctx, Format};
use zipunzip_lib::safety::{Limits, MIB};

fn engine() -> PathBuf {
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("engine");
    let e = Engine::locate(Some(&bundled))
        .expect("7-Zip non trovato: esegui `npm run fetch-7zip` o installa 7-Zip");
    eprintln!(
        "motore: {} ({}, incluso: {})",
        e.path.display(),
        e.version,
        e.bundled
    );
    e.path
}

fn ctx<'a>(engine: &'a Path, cancel: &'a AtomicBool) -> Ctx<'a> {
    Ctx {
        engine,
        cancel,
        progress: &|_, _| {},
        limits: Limits::default(),
    }
}

fn opts(format: Format) -> CompressOptions {
    CompressOptions {
        format,
        level: 5,
        password: None,
        encrypt_names: false,
        exclude_system_files: true,
    }
}

/// Albero di prova con nomi Unicode, spazi, trattino iniziale e cartella vuota.
fn sample_tree(root: &Path) -> PathBuf {
    let base = root.join("Progetto è più");
    fs::create_dir_all(base.join("sotto cartella/livello 2")).unwrap();
    fs::create_dir_all(base.join("vuota")).unwrap();
    fs::write(base.join("leggimi.txt"), "Ciao dal mondo pezzaliAPP\n").unwrap();
    fs::write(base.join("-trattino.txt"), "opzione?").unwrap();
    fs::write(base.join("sotto cartella/日本語.txt"), "こんにちは").unwrap();
    let big: Vec<u8> = (0..2_000_000u32).map(|i| (i % 251) as u8).collect();
    fs::write(base.join("sotto cartella/livello 2/dati.bin"), big).unwrap();
    fs::write(base.join(".DS_Store"), "junk").unwrap();
    base
}

/// Confronta due alberi: stessi file con stesso contenuto (ignora .DS_Store).
fn assert_same_tree(a: &Path, b: &Path) {
    let collect = |root: &Path| {
        let mut v: Vec<(String, Option<Vec<u8>>)> = walkdir::WalkDir::new(root)
            .min_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_name() != ".DS_Store")
            .map(|e| {
                let rel = e
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let data = e.file_type().is_file().then(|| fs::read(e.path()).unwrap());
                (rel, data)
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(
        collect(a),
        collect(b),
        "contenuto diverso tra {} e {}",
        a.display(),
        b.display()
    );
}

fn hash_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut v: Vec<_> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| (e.path().to_path_buf(), fs::read(e.path()).unwrap()))
        .collect();
    v.sort();
    v
}

fn no_temp_left(dir: &Path) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        assert!(
            !e.file_name().to_string_lossy().starts_with(".pezzaliapp-"),
            "cartella temporanea rimasta: {}",
            e.path().display()
        );
    }
}

#[test]
fn roundtrip_all_formats() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let src = sample_tree(work.path());
    let before = hash_tree(&src);

    for format in [
        Format::SevenZ,
        Format::Zip,
        Format::Tar,
        Format::TarGz,
        Format::TarXz,
    ] {
        let out_dir = work.path().join(format!("out-{format:?}"));
        fs::create_dir(&out_dir).unwrap();
        let r = ops::compress(
            &c,
            std::slice::from_ref(&src),
            &out_dir,
            "prova",
            &opts(format),
        )
        .unwrap();
        assert!(
            r.output.to_string_lossy().ends_with(format.extension()),
            "{format:?}"
        );
        assert!(r.size > 0);

        let listing = ops::list(&c, &r.output, None).unwrap();
        assert!(
            listing.entries.iter().any(|e| e.path.ends_with("dati.bin")),
            "{format:?}"
        );
        assert!(
            !listing
                .entries
                .iter()
                .any(|e| e.path.ends_with(".DS_Store")),
            "{format:?}: .DS_Store incluso"
        );

        let t = ops::test(&c, &r.output, None).unwrap();
        assert!(t.ok);

        let ex_dir = out_dir.join("estratto");
        fs::create_dir(&ex_dir).unwrap();
        let x = ops::extract(&c, &r.output, &ex_dir, None, false).unwrap();
        assert_eq!(x.output, ex_dir.join("Progetto è più"), "{format:?}");
        assert_same_tree(&src, &x.output);
        assert!(x.warnings.is_empty(), "{format:?}: {:?}", x.warnings);
        no_temp_left(&out_dir);
        no_temp_left(&ex_dir);
    }
    assert_eq!(
        before,
        hash_tree(&src),
        "i file originali sono stati modificati"
    );
}

#[test]
fn never_overwrites() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let src = sample_tree(work.path());
    let out = work.path().join("out");
    fs::create_dir(&out).unwrap();

    let a = ops::compress(
        &c,
        std::slice::from_ref(&src),
        &out,
        "doppio.zip",
        &opts(Format::Zip),
    )
    .unwrap();
    let b = ops::compress(
        &c,
        std::slice::from_ref(&src),
        &out,
        "doppio.zip",
        &opts(Format::Zip),
    )
    .unwrap();
    assert_eq!(a.output, out.join("doppio.zip"));
    assert_eq!(b.output, out.join("doppio (2).zip"));

    let first = ops::extract(&c, &a.output, &out, None, false).unwrap();
    fs::write(first.output.join("leggimi.txt"), "modificato dall'utente").unwrap();
    let second = ops::extract(&c, &a.output, &out, None, false).unwrap();
    assert_eq!(second.output, out.join("Progetto è più (2)"));
    assert_eq!(
        fs::read_to_string(first.output.join("leggimi.txt")).unwrap(),
        "modificato dall'utente"
    );
}

#[test]
fn multiple_top_level_items_get_own_folder() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("uno.txt"), "1").unwrap();
    fs::write(work.path().join("due.txt"), "2").unwrap();
    let r = ops::compress(
        &c,
        &[work.path().join("uno.txt"), work.path().join("due.txt")],
        work.path(),
        "coppia",
        &opts(Format::SevenZ),
    )
    .unwrap();
    let dest = work.path().join("dest");
    fs::create_dir(&dest).unwrap();
    let x = ops::extract(&c, &r.output, &dest, None, false).unwrap();
    assert_eq!(x.output, dest.join("coppia"));
    assert_eq!(fs::read_to_string(x.output.join("due.txt")).unwrap(), "2");
    assert_eq!(x.files, 2);
}

#[test]
fn passwords_7z_and_zip() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let src = sample_tree(work.path());
    let unicode_pw = "Pàssw0rd con spazi €";
    let ascii_pw = "Passw0rd con spazi!";

    let mut o = opts(Format::Zip);
    o.password = Some(unicode_pw.into());
    let e = ops::compress(&c, std::slice::from_ref(&src), work.path(), "z", &o).unwrap_err();
    assert_eq!(e.code, "zip-password-ascii");
    no_temp_left(work.path());

    for (format, encrypt_names) in [
        (Format::SevenZ, true),
        (Format::SevenZ, false),
        (Format::Zip, false),
    ] {
        let pw = if format == Format::Zip {
            ascii_pw
        } else {
            unicode_pw
        };
        let mut o = opts(format);
        o.password = Some(pw.into());
        o.encrypt_names = encrypt_names;
        let r = ops::compress(&c, std::slice::from_ref(&src), work.path(), "segreto", &o).unwrap();

        if encrypt_names {
            let e = ops::list(&c, &r.output, None).unwrap_err();
            assert_eq!(e.code, "need-password", "{e}");
            let e = ops::list(&c, &r.output, Some("sbagliata")).unwrap_err();
            assert_eq!(e.code, "wrong-password", "{e}");
        } else {
            let l = ops::list(&c, &r.output, None).unwrap();
            assert!(l.any_encrypted(), "{format:?}");
        }

        let dest = work.path().join(format!("dest-{format:?}-{encrypt_names}"));
        fs::create_dir(&dest).unwrap();
        let e = ops::extract(&c, &r.output, &dest, None, false).unwrap_err();
        assert_eq!(e.code, "need-password", "{format:?}: {e}");
        let e = ops::extract(&c, &r.output, &dest, Some("sbagliata"), false).unwrap_err();
        assert_eq!(e.code, "wrong-password", "{format:?}: {e}");
        no_temp_left(&dest);
        assert_eq!(
            fs::read_dir(&dest).unwrap().count(),
            0,
            "file parziali lasciati dopo password errata"
        );

        let x = ops::extract(&c, &r.output, &dest, Some(pw), false).unwrap();
        assert_same_tree(&src, &x.output);
        assert!(ops::test(&c, &r.output, Some(pw)).unwrap().ok);
        fs::remove_file(&r.output).unwrap();
    }
}

#[test]
fn password_rejected_for_tar() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let src = sample_tree(work.path());
    let mut o = opts(Format::TarGz);
    o.password = Some("x".into());
    assert_eq!(
        ops::compress(&c, &[src], work.path(), "a", &o)
            .unwrap_err()
            .code,
        "bad-option"
    );
    no_temp_left(work.path());
}

// ------------------------------------------------------------ archivi malevoli

/// ZIP minimale (metodo "stored") con nomi arbitrari, anche pericolosi.
fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let n = name.as_bytes();
        let mut local = Vec::new();
        local.extend_from_slice(&0x04034b50u32.to_le_bytes());
        local.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(n.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(n);
        out.extend_from_slice(&local);
        out.extend_from_slice(data);

        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(n.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(n);
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    fs::write(path, out).unwrap();
}

/// TAR ustar minimale: (nome, tipo '0' file / '2' symlink / '1' hardlink, link, dati).
#[cfg(unix)]
fn write_tar(path: &Path, items: &[(&str, u8, &str, &[u8])]) {
    let mut out = Vec::new();
    for (name, kind, link, data) in items {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[100..107].copy_from_slice(b"0000644");
        h[108..115].copy_from_slice(b"0000000");
        h[116..123].copy_from_slice(b"0000000");
        let size = format!("{:011o}", data.len());
        h[124..135].copy_from_slice(size.as_bytes());
        h[136..147].copy_from_slice(b"00000000000");
        h[156] = *kind;
        h[157..157 + link.len()].copy_from_slice(link.as_bytes());
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        h[148..156].copy_from_slice(b"        ");
        let sum: u32 = h.iter().map(|b| *b as u32).sum();
        h[148..155].copy_from_slice(format!("{sum:06o}\0").as_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(512) * 512, 0);
    }
    out.extend_from_slice(&[0u8; 1024]);
    fs::write(path, out).unwrap();
}

fn assert_nothing_escaped(sandbox: &Path, dest: &Path) {
    for e in walkdir::WalkDir::new(sandbox)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let p = e.path();
        if p.starts_with(dest) || p == sandbox {
            continue;
        }
        let name = e.file_name().to_string_lossy();
        assert!(
            !["evil.txt", "evil2.txt", "pwn.txt", "outside"].contains(&name.as_ref()),
            "file scritto fuori dalla destinazione: {}",
            p.display()
        );
    }
}

#[test]
fn zip_slip_is_blocked() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let sandbox = tempfile::tempdir().unwrap();
    let dest = sandbox.path().join("a/b/dest");
    fs::create_dir_all(&dest).unwrap();
    let zip = sandbox.path().join("slip.zip");
    write_zip(
        &zip,
        &[
            ("../../evil.txt", b"x"),
            ("/tmp/abs/evil2.txt", b"y"),
            ("ok/fine.txt", b"z"),
            ("ok/../../../evil.txt", b"w"),
        ],
    );

    let e = ops::extract(&c, &zip, &dest, None, false).unwrap_err();
    assert_eq!(e.code, "unsafe");
    let kinds: Vec<_> = e.issues.iter().map(|i| i.kind.as_str()).collect();
    // 7-Zip normalizza già i percorsi assoluti nell'elenco: resta il traversal.
    assert!(kinds.contains(&"path-traversal"), "{kinds:?}");
    assert_eq!(
        fs::read_dir(&dest).unwrap().count(),
        0,
        "nulla deve essere estratto"
    );

    // Anche accettando il rischio, nulla esce dalla destinazione.
    let x = ops::extract(&c, &zip, &dest, None, true).unwrap();
    assert!(x.output.starts_with(&dest));
    assert!(!x.warnings.is_empty());
    assert_nothing_escaped(sandbox.path(), &dest);
    no_temp_left(&dest);
}

#[cfg(unix)]
#[test]
fn symlink_attacks_are_blocked() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let sandbox = tempfile::tempdir().unwrap();
    let dest = sandbox.path().join("a/b/dest");
    fs::create_dir_all(&dest).unwrap();
    let outside_dir = sandbox.path().join("bersaglio");
    fs::create_dir(&outside_dir).unwrap();
    let tar = sandbox.path().join("link.tar");
    let abs_target = outside_dir.to_string_lossy().to_string();
    write_tar(
        &tar,
        &[
            ("lnk", b'2', &abs_target, b""),
            ("lnk/pwn.txt", b'0', "", b"pwn"),
            ("rel", b'2', "../../outside", b""),
            ("hard", b'1', "../../../bersaglio/x", b""),
            ("buono", b'2', "ok.txt", b""),
            ("ok.txt", b'0', "", b"ok"),
        ],
    );

    let e = ops::extract(&c, &tar, &dest, None, false).unwrap_err();
    assert_eq!(e.code, "unsafe");
    let kinds: Vec<_> = e.issues.iter().map(|i| i.kind.as_str()).collect();
    assert!(
        kinds.contains(&"unsafe-symlink") && kinds.contains(&"unsafe-hardlink"),
        "{kinds:?}"
    );
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 0);

    let x = ops::extract(&c, &tar, &dest, None, true).unwrap();
    assert_eq!(
        fs::read_dir(&outside_dir).unwrap().count(),
        0,
        "scritto nel bersaglio esterno"
    );
    assert_nothing_escaped(sandbox.path(), &dest);
    // Nessun link estratto punta fuori dalla destinazione.
    let canon_dest = fs::canonicalize(&dest).unwrap();
    for e in walkdir::WalkDir::new(&x.output)
        .into_iter()
        .filter_map(Result::ok)
    {
        if e.file_type().is_symlink() {
            if let Ok(real) = fs::canonicalize(e.path()) {
                assert!(
                    real.starts_with(&canon_dest),
                    "link esterno: {}",
                    e.path().display()
                );
            }
        }
    }
    no_temp_left(&dest);
}

#[test]
fn bomb_ratio_is_detected() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let mut c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let zeros = work.path().join("zeri.bin");
    fs::File::create(&zeros)
        .unwrap()
        .write_all(&vec![0u8; 64 * MIB as usize])
        .unwrap();
    let r = ops::compress(&c, &[zeros], work.path(), "bomba", &opts(Format::SevenZ)).unwrap();

    c.limits = Limits {
        ratio_min_size: 16 * MIB,
        max_ratio: 100,
        ..Limits::default()
    };
    let dest = work.path().join("dest");
    fs::create_dir(&dest).unwrap();
    let e = ops::extract(&c, &r.output, &dest, None, false).unwrap_err();
    assert_eq!(e.code, "unsafe");
    assert_eq!(e.issues[0].kind, "suspicious-ratio");

    c.limits = Limits {
        max_total: 8 * MIB,
        ..Limits::default()
    };
    let e = ops::extract(&c, &r.output, &dest, None, true).unwrap_err();
    assert_eq!(e.code, "too-large");
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 0);
}

#[test]
fn watchdog_kills_engine() {
    // Il watchdog interrompe davvero il processo 7-Zip in corso.
    use zipunzip_lib::engine::{self as eng_mod, os, Input, Stage};
    let eng = engine();
    let work = tempfile::tempdir().unwrap();
    let mut x: u64 = 0x2545F4914F6CDD1D;
    let data: Vec<u8> = (0..128 * MIB as usize)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect();
    let src = work.path().join("dati.bin");
    fs::write(&src, data).unwrap();
    let out = work.path().join("o.7z");
    let cancel = AtomicBool::new(false);
    let mut calls = 0;
    let mut wd = || {
        calls += 1;
        if calls > 3 {
            Err(zipunzip_lib::error::OpError::new("bomb", "test"))
        } else {
            Ok(())
        }
    };
    let start = std::time::Instant::now();
    let args = vec![
        os("a"),
        os("-mx=9"),
        os("-bso0"),
        os("--"),
        os(&out),
        os(&src),
    ];
    let e = eng_mod::run(
        &eng,
        vec![Stage::new(args, Input::Closed)],
        &cancel,
        &|_| {},
        Some(&mut wd),
    )
    .unwrap_err();
    assert_eq!(e.code, "bomb");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "processo non interrotto in tempo"
    );
}

#[test]
fn forged_size_is_stopped() {
    // gzip con dimensione dichiarata falsificata (campo ISIZE): 7-Zip o il
    // watchdog interrompono l'estrazione e non resta alcun file.
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let zeros = work.path().join("zeri.bin");
    fs::File::create(&zeros)
        .unwrap()
        .write_all(&vec![0u8; 160 * MIB as usize])
        .unwrap();
    let out = work.path().join("zeri.bin.gz");
    let st = std::process::Command::new(&eng)
        .args(["a", "-tgzip", "-mx1", "-bso0", "-bsp0"])
        .arg(&out)
        .arg(&zeros)
        .status()
        .unwrap();
    assert!(st.success());
    let mut data = fs::read(&out).unwrap();
    let n = data.len();
    data[n - 4..].copy_from_slice(&1000u32.to_le_bytes());
    fs::write(&out, data).unwrap();

    let dest = work.path().join("dest");
    fs::create_dir(&dest).unwrap();
    let e = ops::extract(&c, &out, &dest, None, false).unwrap_err();
    assert!(
        ["bomb", "corrupt", "engine"].contains(&e.code.as_str()),
        "{e}"
    );
    eprintln!("esito bomba falsificata: {e}");
    assert_eq!(
        fs::read_dir(&dest).unwrap().count(),
        0,
        "file lasciati dopo il blocco"
    );
}

#[test]
fn cancel_cleans_up() {
    let eng = engine();
    let work = tempfile::tempdir().unwrap();
    let src = work.path().join("grande.bin");
    // Dati poco comprimibili per rendere l'operazione abbastanza lunga.
    let mut x: u64 = 0x9E3779B97F4A7C15;
    let data: Vec<u8> = (0..256 * MIB as usize)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect();
    fs::write(&src, data).unwrap();
    let out = work.path().join("out");
    fs::create_dir(&out).unwrap();

    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let t = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        flag.store(true, Ordering::SeqCst);
    });
    let c = ctx(&eng, &cancel);
    let mut o = opts(Format::SevenZ);
    o.level = 9;
    let e = ops::compress(&c, std::slice::from_ref(&src), &out, "annullato", &o).unwrap_err();
    t.join().unwrap();
    assert_eq!(e.code, "cancelled");
    assert_eq!(
        fs::read_dir(&out).unwrap().count(),
        0,
        "file parziali lasciati"
    );
    assert!(src.is_file(), "l'originale deve restare");
}

#[test]
fn not_an_archive() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let f = work.path().join("testo.zip");
    fs::write(&f, "questo non è un archivio").unwrap();
    let e = ops::list(&c, &f, None).unwrap_err();
    assert_eq!(e.code, "not-archive", "{e}");
    let e = ops::extract(&c, &work.path().join("manca.zip"), work.path(), None, false).unwrap_err();
    assert_eq!(e.code, "not-found");
}

/// Esegue uno strumento di sistema; `None` se non è installato.
#[cfg(unix)]
fn system_tool(cmd: &str, args: &[&std::ffi::OsStr], cwd: &Path) -> Option<bool> {
    match std::process::Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
    {
        Ok(o) => Some(o.status.success()),
        Err(_) => {
            eprintln!("SALTATO: strumento di sistema '{cmd}' non disponibile");
            None
        }
    }
}

#[cfg(unix)]
#[test]
fn interoperability_with_system_tools() {
    let eng = engine();
    let cancel = AtomicBool::new(false);
    let c = ctx(&eng, &cancel);
    let work = tempfile::tempdir().unwrap();
    let src = sample_tree(work.path());
    let os = |s: &str| std::ffi::OsString::from(s);

    // App → strumenti di sistema
    let z = ops::compress(
        &c,
        std::slice::from_ref(&src),
        work.path(),
        "interop",
        &opts(Format::Zip),
    )
    .unwrap();
    let out_unzip = work.path().join("unzip-out");
    fs::create_dir(&out_unzip).unwrap();
    if let Some(ok) = system_tool(
        "unzip",
        &[
            &os("-q"),
            z.output.as_os_str(),
            &os("-d"),
            out_unzip.as_os_str(),
        ],
        work.path(),
    ) {
        assert!(ok, "unzip non riesce ad aprire lo ZIP creato dall'app");
        assert_same_tree(&src, &out_unzip.join("Progetto è più"));
    }
    let tgz = ops::compress(
        &c,
        std::slice::from_ref(&src),
        work.path(),
        "interop",
        &opts(Format::TarGz),
    )
    .unwrap();
    let out_tar = work.path().join("tar-out");
    fs::create_dir(&out_tar).unwrap();
    if let Some(ok) = system_tool(
        "tar",
        &[
            &os("-xzf"),
            tgz.output.as_os_str(),
            &os("-C"),
            out_tar.as_os_str(),
        ],
        work.path(),
    ) {
        assert!(ok, "tar non riesce ad aprire il TAR.GZ creato dall'app");
        assert_same_tree(&src, &out_tar.join("Progetto è più"));
    }

    // Strumenti di sistema → app
    let name = std::ffi::OsString::from("Progetto è più");
    let sys_tgz = work.path().join("sistema.tar.gz");
    if system_tool(
        "tar",
        &[&os("-czf"), sys_tgz.as_os_str(), &name],
        work.path(),
    ) == Some(true)
    {
        let dest = work.path().join("da-tar");
        fs::create_dir(&dest).unwrap();
        let x = ops::extract(&c, &sys_tgz, &dest, None, false).unwrap();
        assert_same_tree(&src, &x.output);
    }
    let sys_zip = work.path().join("sistema.zip");
    if system_tool(
        "zip",
        &[&os("-qr"), sys_zip.as_os_str(), &name],
        work.path(),
    ) == Some(true)
    {
        let dest = work.path().join("da-zip");
        fs::create_dir(&dest).unwrap();
        let x = ops::extract(&c, &sys_zip, &dest, None, false).unwrap();
        assert_same_tree(&src, &x.output);
    }
}
