# Architettura

```
┌──────────────── Interfaccia (WebView di sistema) ────────────────┐
│ React 19 + TypeScript                                             │
│ App.tsx ─ schede Estrai/Comprimi, trascinamento, avanzamento      │
│ components/ExtractView, CompressView, common, AboutDialog         │
│ api.ts  ─ invoke() tipizzato + evento "job-progress"              │
└──────────────────────────────┬───────────────────────────────────┘
                               │ IPC Tauri (solo comandi dell'app;
                               │ permessi: dialog:open, opener:reveal)
┌──────────────────────────────▼───────────────────────────────────┐
│ Rust (src-tauri/src)                                              │
│ lib.rs     comandi Tauri, registro lavori, annullamento, eventi   │
│ ops.rs     list / extract / compress / test (senza dipendenze UI) │
│ safety.rs  analisi preventiva, verifica post-estrazione, nomi     │
│ listing.rs parser di `7z l -slt`                                  │
│ engine.rs  ricerca del motore, processi, pipeline, errori         │
│ error.rs   errore con codice stabile tradotto dall'interfaccia    │
└──────────────────────────────┬───────────────────────────────────┘
                               │ processo separato, argomenti come
                               │ vettore (niente shell), password su stdin
┌──────────────────────────────▼───────────────────────────────────┐
│ 7-Zip ufficiale (7zz / 7z.exe + 7z.dll) — LGPL, non modificato    │
│ cercato in: Resources/engine → PATH → percorsi standard           │
└───────────────────────────────────────────────────────────────────┘
```

## Flusso di estrazione

1. `7z l -slt` → elenco; per `.tar.gz`/`.tar.xz`/… l'elenco interno si legge
   con una pipeline `7z x -so | 7z l -si -ttar`.
2. `safety::scan_listing`: percorsi assoluti, `..`, link verso l'esterno,
   rapporto di compressione. Se ci sono problemi l'estrazione si ferma e
   l'utente vede l'elenco; può proseguire in "modo protetto".
3. Controlli: password, dimensione massima (64 GB), spazio libero.
4. Estrazione in `destinazione/.pezzaliapp-XXXXXX` (cartella temporanea
   creata con `tempfile`, permessi 0700) con `-aou` (nessuna domanda, nessuna
   sovrascrittura). Un watchdog misura la dimensione reale e lo spazio libero
   e interrompe 7-Zip se necessario.
5. `safety::post_scan`: rimuove link esterni, bit setuid/setgid/sticky,
   separa hard link condivisi con file esterni; su macOS propaga la quarantena.
6. Spostamento finale: un solo elemento di primo livello viene spostato così
   com'è; più elementi finiscono in una cartella con il nome dell'archivio.
   Il nome è sempre libero (`nome (2)`). In caso di errore o annullamento la
   cartella temporanea viene eliminata (contiene solo dati estratti dall'app).

## Flusso di compressione

`7z a` scrive in una cartella temporanea nella destinazione; `.tar.gz` e
`.tar.xz` usano `7z a -ttar -so | 7z a -tgzip|-txz -si`. Al termine il file
viene spostato con un nome libero. Opzioni: `-snl` (i link restano link, non
si includono file esterni alla selezione), esclusione dei file di sistema,
`-mhe=on` per cifrare i nomi in 7z, `-mem=AES256` per ZIP.

## Scelte

- **CLI anziché libreria**: mantiene separata la licenza LGPL e usa il binario
  ufficiale senza modifiche.
- **Nessuna shell**: `std::process::Command` con argomenti separati e `--`
  prima dei nomi di file (file che iniziano con `-` sono gestiti).
- **Password su stdin**: non visibile in `ps`/Gestione attività.
- **Codici di errore stabili** (`need-password`, `unsafe`, `bomb`, …) tradotti
  dall'interfaccia in italiano/inglese.
- **Elenco limitato a 2000 voci** verso l'interfaccia, con totali completi.
- **`--self-test`**: l'eseguibile stampa il motore trovato ed esce (diagnostica).
