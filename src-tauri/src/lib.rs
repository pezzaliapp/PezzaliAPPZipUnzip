//! PezzaliAPPZipUnzip — © 2026 Alessandro Pezzali — MIT.
//! Il motore 7-Zip è un programma separato con licenza propria
//! (LGPL-2.1 / BSD, restrizione unRAR): vedi THIRD_PARTY_NOTICES.md.

pub mod engine;
pub mod error;
pub mod listing;
pub mod ops;
pub mod safety;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use engine::Engine;
use error::OpError;
use ops::{CompressOptions, Ctx};
use safety::Limits;

#[derive(Default)]
struct AppState {
    engine: Mutex<Option<Engine>>,
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent<'a> {
    job_id: &'a str,
    phase: &'a str,
    percent: u8,
}

fn bundled_engine_dir<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().resource_dir().ok().map(|d| d.join("engine"))
}

fn get_engine<R: Runtime>(app: &AppHandle<R>, state: &AppState) -> Result<Engine, OpError> {
    let mut guard = state.engine.lock().unwrap();
    if let Some(e) = guard.as_ref() {
        return Ok(e.clone());
    }
    let found = Engine::locate(bundled_engine_dir(app).as_deref()).ok_or_else(|| {
        OpError::new(
            "engine-missing",
            "Motore 7-Zip non trovato. Installa 7-Zip oppure usa il pacchetto completo.",
        )
    })?;
    *guard = Some(found.clone());
    Ok(found)
}

/// Esegue un'operazione lunga in un thread dedicato, con annullamento
/// e notifiche di avanzamento verso l'interfaccia.
async fn run_job<R: Runtime, T, F>(app: AppHandle<R>, job_id: String, f: F) -> Result<T, OpError>
where
    T: Send + 'static,
    F: FnOnce(&Ctx) -> Result<T, OpError> + Send + 'static,
{
    let state = app.state::<AppState>();
    let engine = get_engine(&app, &state)?;
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .jobs
        .lock()
        .unwrap()
        .insert(job_id.clone(), Arc::clone(&cancel));

    let app2 = app.clone();
    let id2 = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let progress = |phase: &str, percent: u8| {
            let _ = app2.emit(
                "job-progress",
                ProgressEvent {
                    job_id: &id2,
                    phase,
                    percent,
                },
            );
        };
        let ctx = Ctx {
            engine: &engine.path,
            cancel: &cancel,
            progress: &progress,
            limits: Limits::default(),
        };
        f(&ctx)
    })
    .await
    .unwrap_or_else(|e| Err(OpError::engine(format!("errore interno: {e}"))));

    state.jobs.lock().unwrap().remove(&job_id);
    result
}

#[tauri::command]
fn engine_info<R: Runtime>(app: AppHandle<R>, state: State<AppState>) -> Result<Engine, OpError> {
    get_engine(&app, &state)
}

/// Voci massime inviate all'interfaccia: l'anteprima non ne mostra di più
/// e archivi con milioni di file produrrebbero messaggi enormi.
const UI_ENTRY_LIMIT: usize = 2000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListingSummary {
    #[serde(flatten)]
    listing: listing::Listing,
    total_files: usize,
    total_dirs: usize,
    total_size: u64,
    encrypted: bool,
    truncated: bool,
}

#[tauri::command]
async fn list_archive<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
    archive: PathBuf,
    password: Option<String>,
) -> Result<ListingSummary, OpError> {
    let mut l = run_job(app, job_id, move |ctx| {
        ops::list(ctx, &archive, password.as_deref())
    })
    .await?;
    let total_files = l.entries.iter().filter(|e| !e.is_dir).count();
    let summary = ListingSummary {
        total_dirs: l.entries.len() - total_files,
        total_files,
        total_size: l.total_size(),
        encrypted: l.any_encrypted(),
        truncated: l.entries.len() > UI_ENTRY_LIMIT,
        listing: {
            l.entries.truncate(UI_ENTRY_LIMIT);
            l
        },
    };
    Ok(summary)
}

#[tauri::command]
async fn extract_archive<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
    archive: PathBuf,
    dest_dir: PathBuf,
    password: Option<String>,
    allow_unsafe: bool,
) -> Result<ops::ExtractResult, OpError> {
    run_job(app, job_id, move |ctx| {
        ops::extract(ctx, &archive, &dest_dir, password.as_deref(), allow_unsafe)
    })
    .await
}

#[tauri::command]
async fn compress_files<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
    sources: Vec<PathBuf>,
    out_dir: PathBuf,
    name: String,
    options: CompressOptions,
) -> Result<ops::CompressResult, OpError> {
    run_job(app, job_id, move |ctx| {
        ops::compress(ctx, &sources, &out_dir, &name, &options)
    })
    .await
}

#[tauri::command]
async fn test_archive<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
    archive: PathBuf,
    password: Option<String>,
) -> Result<ops::TestResult, OpError> {
    run_job(app, job_id, move |ctx| {
        ops::test(ctx, &archive, password.as_deref())
    })
    .await
}

#[tauri::command]
fn cancel_job(state: State<AppState>, job_id: String) {
    if let Some(flag) = state.jobs.lock().unwrap().get(&job_id) {
        flag.store(true, Ordering::SeqCst);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PathInfo {
    path: PathBuf,
    name: String,
    is_dir: bool,
    size: u64,
}

/// Informazioni minime sui percorsi trascinati nella finestra.
#[tauri::command]
fn path_info(paths: Vec<PathBuf>) -> Vec<PathInfo> {
    paths
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::symlink_metadata(&p).ok()?;
            let name = p.file_name()?.to_string_lossy().to_string();
            let size = if m.is_dir() {
                safety::tree_size(&p)
            } else {
                m.len()
            };
            Some(PathInfo {
                is_dir: m.is_dir(),
                size,
                name,
                path: p,
            })
        })
        .collect()
}

/// File passati all'avvio (es. "Apri con" su Windows e Linux).
#[tauri::command]
fn launch_paths() -> Vec<PathBuf> {
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.exists())
        .collect()
}

/// Stato e comandi dell'app, separati dai plugin per poterli testare.
pub fn with_commands<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            engine_info,
            list_archive,
            extract_archive,
            compress_files,
            test_archive,
            cancel_job,
            path_info,
            launch_paths
        ])
}

/// Configurazione e autorizzazioni generate da tauri.conf.json (una sola volta).
fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    with_commands(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Diagnostica: `--self-test` stampa il motore trovato ed esce.
            if std::env::args().any(|a| a == "--self-test") {
                let state = app.state::<AppState>();
                let code = match get_engine(app.handle(), &state) {
                    Ok(e) => {
                        println!(
                            "PezzaliAPPZipUnzip {} — 7-Zip {} ({}) {}",
                            env!("CARGO_PKG_VERSION"),
                            e.version,
                            if e.bundled { "incluso" } else { "di sistema" },
                            e.path.display()
                        );
                        0
                    }
                    Err(e) => {
                        eprintln!("PezzaliAPPZipUnzip: {e}");
                        1
                    }
                };
                std::process::exit(code);
            }
            Ok(())
        })
        .run(context())
        .expect("errore durante l'avvio di PezzaliAPPZipUnzip");
}

#[cfg(test)]
mod ipc_tests {
    //! Invoca i comandi attraverso l'IPC di Tauri con gli stessi payload
    //! JSON inviati dall'interfaccia (src/api.ts).
    use serde_json::{json, Value};
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
    use tauri::webview::InvokeRequest;
    use tauri::WebviewWindowBuilder;

    fn invoke(
        w: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        cmd: &str,
        body: Value,
    ) -> Result<Value, Value> {
        let req = InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: w.url().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        };
        get_ipc_response(w, req).map(|r| r.deserialize::<Value>().unwrap())
    }

    #[test]
    fn commands_roundtrip_over_ipc() {
        // Contesto reale: include la configurazione e le autorizzazioni (ACL) dell'app.
        let app = super::with_commands(mock_builder())
            .build(super::context())
            .unwrap();
        let w = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let engine = invoke(&w, "engine_info", json!({})).expect("motore 7-Zip non trovato");
        assert!(engine["version"].as_str().unwrap().contains('.'));

        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("documento.txt");
        std::fs::write(&src, "contenuto di prova").unwrap();

        let info = invoke(&w, "path_info", json!({ "paths": [src] })).unwrap();
        assert_eq!(info[0]["name"], "documento.txt");
        assert_eq!(info[0]["isDir"], false);

        for format in ["7z", "zip", "tar", "tar-gz", "tar-xz"] {
            let r = invoke(&w, "compress_files", json!({
                "jobId": "j1", "sources": [src], "outDir": dir.path(), "name": format!("prova-{format}"),
                "options": { "format": format, "level": 5, "password": null, "encryptNames": false, "excludeSystemFiles": true }
            }))
            .unwrap_or_else(|e| panic!("{format}: {e}"));
            let archive = r["output"].as_str().unwrap().to_string();

            let l = invoke(
                &w,
                "list_archive",
                json!({ "jobId": "j2", "archive": archive, "password": null }),
            )
            .unwrap();
            assert_eq!(l["entries"][0]["path"], "documento.txt", "{format}");

            let t = invoke(
                &w,
                "test_archive",
                json!({ "jobId": "j3", "archive": archive, "password": null }),
            )
            .unwrap();
            assert_eq!(t["ok"], true);

            let dest = dir.path().join(format!("dest-{format}"));
            std::fs::create_dir(&dest).unwrap();
            let x = invoke(&w, "extract_archive", json!({
                "jobId": "j4", "archive": archive, "destDir": dest, "password": null, "allowUnsafe": false
            }))
            .unwrap();
            assert_eq!(x["files"], 1);
            let out = x["output"].as_str().unwrap();
            assert_eq!(std::fs::read_to_string(out).unwrap(), "contenuto di prova");
        }

        // Gli errori arrivano all'interfaccia come oggetti con codice stabile.
        let e = invoke(&w, "extract_archive", json!({
            "jobId": "j5", "archive": dir.path().join("manca.zip"), "destDir": dir.path(), "password": null, "allowUnsafe": false
        }))
        .unwrap_err();
        assert_eq!(e["code"], "not-found");
        assert!(e["issues"].is_array());

        invoke(&w, "cancel_job", json!({ "jobId": "inesistente" })).unwrap();
    }
}
