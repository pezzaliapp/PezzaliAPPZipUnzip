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
  italiano e inglese.
- **Non eseguito**: interazione dell'utente con la finestra nativa (catture
  dello schermo bloccate dai permessi di sistema, non modificati).

## Non verificato

- Windows e Linux (nessuna macchina o VM avviata; vedi rapporto finale).
- Hardware Intel reale (solo Rosetta 2).
- Estrazione di archivi RAR, ISO, CAB reali (formati gestiti da 7-Zip; nessun
  campione disponibile, la creazione di RAR non è consentita dalla licenza).
- Script `fetch-7zip` eseguito su host Windows (testato da macOS anche per i
  target `win-x64` e `linux-x64`: download, hash ed estrazione riusciti).
