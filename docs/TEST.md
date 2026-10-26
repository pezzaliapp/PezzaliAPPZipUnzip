# Test

Tutti i test usano il **vero motore 7-Zip** (nessun mock nel backend) e
cartelle temporanee isolate. Se 7-Zip non è disponibile i test falliscono:
nessun esito viene simulato.

## Come eseguirli

```bash
npm run fetch-7zip                 # motore incluso (altrimenti viene usato 7-Zip di sistema)
npm test                           # Vitest: funzioni dell'interfaccia
cd src-tauri
cargo test                         # unità + IPC Tauri + integrazione
cargo test -- --nocapture          # mostra motore usato ed esiti diagnostici
cargo clippy --all-targets         # analisi statica
```

## Esiti registrati — 25 ottobre 2026, macOS 26.6.2 Apple Silicon

Motore: 7-Zip 26.05 ufficiale incluso (`src-tauri/engine/7zz`).

| Suite | Esito |
|---|---|
| Vitest (`src/format.test.ts`) | 6/6 superati |
| Rust unità (`engine`, `listing`, `safety`) | 11/11 superati |
| Rust IPC Tauri (`ipc_tests`, runtime simulato ufficiale `tauri::test`) | 1/1 superato |
| Rust integrazione (`tests/integration.rs`) | 13/13 superati |
| Stessa suite Rust compilata **x86_64** ed eseguita con Rosetta 2 | 25/25 superati |
| Versione precedente della suite d'integrazione (11 test) con 7-Zip 26.01 di sistema (Homebrew), durante lo sviluppo | superati |
| `cargo clippy --all-targets` | 0 avvisi |
| `tsc --noEmit` + `vite build` | nessun errore |

### Test d'integrazione

| Test | Verifica |
|---|---|
| `roundtrip_all_formats` | 7z, ZIP, TAR, TAR.GZ, TAR.XZ: compressione → elenco → verifica → estrazione, contenuto identico (nomi Unicode, spazi, file `-trattino.txt`, cartella vuota, 2 MB binari), `.DS_Store` escluso, originali invariati (hash) |
| `never_overwrites` | secondo archivio → `doppio (2).zip`; seconda estrazione → `… (2)`; modifiche dell'utente preservate |
| `multiple_top_level_items_get_own_folder` | più elementi → cartella con il nome dell'archivio |
| `passwords_7z_and_zip` | 7z con e senza nomi cifrati, ZIP AES-256; password mancante/errata/corretta; nessun file lasciato dopo errore; password ZIP non ASCII rifiutata con messaggio chiaro |
| `password_rejected_for_tar` | TAR non accetta password |
| `zip_slip_is_blocked` | ZIP artigianale con `../../evil.txt`, percorso assoluto, `ok/../../../`: blocco, poi modo protetto senza scritture esterne |
| `symlink_attacks_are_blocked` | TAR artigianale con symlink assoluto, symlink `../../`, file scritto attraverso il link, hard link esterno: blocco, poi nessuna scrittura esterna e nessun link verso l'esterno |
| `bomb_ratio_is_detected` | 64 MB di zeri: rapporto anomalo segnalato; limite di dimensione rispettato |
| `forged_size_is_stopped` | GZIP con dimensione falsificata: interrotto, nessun file lasciato |
| `watchdog_kills_engine` | il watchdog termina davvero il processo 7-Zip in corso |
| `cancel_cleans_up` | annullamento durante la compressione di 256 MB: nessun file parziale, originale intatto |
| `not_an_archive` | file non archivio e file mancante → codici d'errore corretti |
| `interoperability_with_system_tools` | ZIP e TAR.GZ dell'app aperti da `unzip` e `tar` di sistema; archivi di `zip` e `tar` di sistema estratti dall'app |

### Verifiche manuali/strumentali

- **Pacchetti macOS**: `.app` e `.dmg` (arm64 e universale) generati; firma
  ad-hoc verificata con `codesign --verify --deep --strict`; DMG montato in sola
  lettura; app avviata dal DMG senza errori.
- **`--self-test`** sull'app impacchettata: trova `7-Zip 26.05 (incluso)` in
  `Contents/Resources/engine/7zz`, sia nativo arm64 sia x86_64 (Rosetta).
- **Interfaccia**: resa verificata in Chrome headless con IPC Tauri simulato
  (harness temporaneo fuori dal repository): stati iniziale, archivio aperto,
  password, avviso di sicurezza, avanzamento, risultato; tema chiaro e scuro;
  italiano e inglese. Dopo la rimozione del badge del motore dall'intestazione
  (25 ottobre 2026) verificate di nuovo la schermata principale e la finestra
  Informazioni con le attribuzioni di 7-Zip, in italiano e inglese.
- **Non eseguito**: interazione dell'utente con la finestra nativa (catture
  dello schermo bloccate dai permessi di sistema, non modificati).

### Pacchetto macOS rigenerato — 25 ottobre 2026 (commit d950160 + correzioni)

| Verifica | Esito |
|---|---|
| `npm run tauri build -- --target universal-apple-darwin` | `.app` 13,68 MiB, `.dmg` 5,93 MiB |
| Interfaccia incorporata | il binario contiene `assets/index-CS71IGQU.js`, nome dato dall'hash del contenuto del build che include la nuova sezione Informazioni e non il badge del motore |
| `hdiutil verify` del DMG | checksum valido |
| `codesign --verify --deep --strict` (DMG montato in sola lettura) | valida, firma ad-hoc |
| `--self-test` arm64 e x86_64 (Rosetta) dal DMG | `7-Zip 26.05 (incluso)` |
| `7zz` nel pacchetto | identico byte per byte a quello estratto dall'archivio ufficiale verificato (SHA-256) |
| Licenze nel pacchetto | `LICENSE` (MIT), `THIRD_PARTY_NOTICES.md`, `engine/License.txt` (identico all'originale), `engine/readme.txt`, `engine/VERSION.txt` |
| Avvio dell'app dal DMG | processo attivo dopo 4 s, nessun errore |
| Dati personali | prima della correzione il binario conteneva 559 percorsi `/Users/<utente>/.cargo/…`: ora 0 (`--remap-path-prefix` in `scripts/tauri.mjs`); nessun indirizzo email, chiave o token (le occorrenze di "secret"/"token" sono nel dizionario Brotli) |
| File di sviluppo | rimosso dal pacchetto `engine/README.md`; il DMG contiene solo l'app, il link ad Applicazioni, l'icona del volume e il `.DS_Store` del layout (senza percorsi) |
| Test dopo le correzioni | Vitest 6/6, Rust 12/12 + 13/13, clippy 0 avvisi, `cargo fmt --check` ok |
| Build senza motore | `npm run tauri build` si ferma con un messaggio chiaro |

## Linux — 25-26 ottobre 2026, container Ubuntu 24.04 su Colima (Mac Apple Silicon)

Procedura: `scripts/linux-docker.sh <arch>` (build e test) e
`ci/linux/test-package.sh` (installazione del `.deb` in un Ubuntu 24.04 pulito,
`--self-test`, interfaccia su schermo virtuale Xvfb). Immagine ufficiale
`ubuntu:24.04` fissata per digest; Node.js 24.19.0 con SHA-256 da
`SHASUMS256.txt` firmato (firma GPG verificata); Rust 1.96.0 con rustup.

**arm64** è eseguito in modo nativo (VM `vz` arm64). **x86_64 (amd64)** è
eseguito tramite **emulazione Rosetta** nella VM: la build e i test sono reali
(binari x86_64 compilati ed eseguiti), ma non su hardware x86_64.

| Verifica | arm64 (nativo) | amd64 (Rosetta) |
|---|---|---|
| `tsc --noEmit` | ✅ | ✅ |
| Vitest | ✅ 6/6 | ✅ 6/6 |
| Rust unità + IPC | ✅ 12/12 | ✅ 12/12 |
| Integrazione con 7-Zip reale, sicurezza inclusa | ✅ 13/13 | ✅ 13/13 |
| `clippy --all-targets -D warnings` | ✅ 0 avvisi | ✅ 0 avvisi |
| Build release + `.deb` | ✅ | ✅ |
| `--self-test` del binario compilato (Xvfb) | ✅ `7-Zip 26.05 (incluso)` | ✅ `7-Zip 26.05 (incluso)` |
| `.deb` installato con `apt` in Ubuntu pulito | ✅ | ✅ |
| `--self-test` dal percorso installato | ✅ `/usr/lib/PezzaliAPPZipUnzip/engine/7zz` | ✅ idem |
| Interfaccia WebKitGTK su Xvfb | ✅ finestra 960×680, schermata principale e Informazioni catturate, nessun errore nel log | ✅ idem |
| `7zz` nel `.deb` identico all'eseguibile ufficiale (SHA-256) | ✅ `17925963…7415` | ✅ `d32657a7…a886` |
| Licenze nel `.deb` | ✅ `LICENSE`, `THIRD_PARTY_NOTICES.md`, `engine/License.txt`, `readme.txt`, `VERSION.txt` | ✅ idem |
| Percorsi personali (`/Users`, nome utente, email, `/root`, `/work`, `/home`) | ✅ 0 | ✅ 0 |
| Credenziali (chiavi private, token) | ✅ nessuna | ✅ nessuna |

Pacchetti (cartella `linux-out/`, ignorata da git):
- `linux-out/arm64/PezzaliAPPZipUnzip_1.0.0_arm64.deb` — SHA-256 `a5945265d20fca42115461b6c5f8b14f326e99225ea7a6bea20a36cdd6560aa3`
- `linux-out/amd64/PezzaliAPPZipUnzip_1.0.0_amd64.deb` — SHA-256 `22324310aaf416ed6ffddccb73c56e735e7b09283194c792b3df4027aa54ce58`

Note:
- Gli unici messaggi nel log sono avvisi `libEGL … DRI3` dovuti all'assenza di
  accelerazione grafica in Xvfb.
- Con 680 px di altezza la finestra Informazioni richiede lo scorrimento.
- Problemi trovati e corretti durante la verifica: test IPC che su macOS
  passava solo grazie al 7-Zip di Homebrew (ora usa il motore del progetto);
  locale POSIX nel container (ora `C.UTF-8`, come su un desktop).
- **AppImage escluso**: generato in prova, poi scartato (motivi in
  [MULTIPIATTAFORMA.md](MULTIPIATTAFORMA.md)); i file generati sono stati eliminati.

## Windows x64 — 26 ottobre 2026, GitHub Actions (runner standard)

Run [40375818036](https://github.com/pezzaliapp/PezzaliAPPZipUnzip/actions/runs/40375818036)
sul commit `bd9f69d`: runner `windows-latest` (Windows Server 2025, x64,
immagine `windows-2025-vs2026`), Node 24, Rust stable 1.99.1. Minuti fatturati
dal servizio: 0 (repository pubblico).

| Verifica | Esito |
|---|---|
| `fetch-7zip` su host Windows (installer ufficiale aperto con `7zr.exe`, SHA-256 verificato) | ✅ |
| `tsc`, Vitest | ✅ 6/6 |
| Rust unità + IPC | ✅ 11/11 (`post_scan_neutralizes` è solo Unix) |
| Integrazione con 7-Zip reale, sicurezza inclusa | ✅ 11/11 (link simbolici e interoperabilità con `zip`/`tar` di sistema sono test solo Unix) |
| `clippy -D warnings` | ✅ |
| Build: installer NSIS `…_x64-setup.exe` e MSI | ✅ entrambi prodotti |
| `--self-test` del binario compilato | ✅ `7-Zip 26.05 (incluso)` |
| Installer NSIS letto con 7-Zip senza eseguirlo | ✅ contiene `LICENSE`, `THIRD_PARTY_NOTICES.md`, `engine/License.txt`, `readme.txt`, `7z.exe`, `7z.dll` |
| `7z.exe`, `7z.dll`, `License.txt`, `readme.txt` nell'installer | ✅ identici all'installer ufficiale 7-Zip 26.05 (verificato anche in locale) |
| Installazione silenziosa per utente sul runner | ✅ in `%LOCALAPPDATA%\PezzaliAPPZipUnzip` |
| `--self-test` dall'app installata | ✅ `7-Zip 26.05 (incluso)` |
| Percorsi di sviluppo o personali nell'installer | ✅ 0 |

Problema trovato e corretto: i test Rust su Windows terminavano con
`STATUS_ENTRYPOINT_NOT_FOUND` perché il manifest di Windows era incorporato solo
nell'app; ora `build.rs` lo passa al linker per tutti gli eseguibili.

Nello stesso run, **Linux x64 su hardware x86_64 reale** (`ubuntu-24.04`):
Vitest 6/6, Rust 12/12 + 13/13, clippy, `.deb`, `--self-test` ✅.

**Non verificati su Windows**: interazione manuale con l'interfaccia, trascinamento
file, dialoghi nativi, Windows 10, Windows arm64, l'MSI (prodotto ma non
installato: non viene distribuito).

## Non verificato

- Interazione manuale con l'interfaccia su Windows e Linux; Windows arm64.
- Linux su un desktop reale (GNOME/KDE, trascinamento file, dialoghi nativi di
  scelta file, interazione manuale).
- Distribuzioni diverse da Ubuntu 24.04.
- Hardware Intel reale (solo Rosetta 2).
- Estrazione di archivi RAR, ISO, CAB reali (formati gestiti da 7-Zip; nessun
  campione disponibile, la creazione di RAR non è consentita dalla licenza).
