# Rapporto finale — PezzaliAPPZipUnzip 1.0.0

Data: 25 ottobre 2026 · Autore: Alessandro Pezzali · Ecosistema pezzaliAPP

## 1. Esito

App desktop completa e funzionante per comprimere ed estrarre archivi, basata
sul motore ufficiale 7-Zip 26.05, interamente offline e senza costi.
**Verificata su macOS** (Apple Silicon, nativo; Intel tramite Rosetta).
**Windows e Linux sono supportati dal codice ma non ancora verificati.**

## 2. Funzioni realizzate

- Estrazione di tutti i formati letti da 7-Zip, inclusi `.tar.gz/.tar.xz/.tar.bz2/.tar.zst` in un passaggio.
- Compressione in 7z, ZIP, TAR, TAR.GZ, TAR.XZ, cinque livelli.
- Password: AES-256 per 7z (con cifratura dei nomi) e ZIP; sblocco archivi protetti; password solo via stdin.
- Anteprima del contenuto, verifica d'integrità, avanzamento in percentuale, annullamento pulito.
- Trascina e rilascia; apertura di file passati all'avvio (Windows/Linux).
- "Mostra nella cartella" del risultato.
- Sicurezza: analisi preventiva (Zip Slip, percorsi assoluti, link, bomb), cartella temporanea, watchdog, verifica successiva, nessuna sovrascrittura, originali intatti, quarantena macOS propagata.
- Interfaccia originale pezzaliAPP, italiano/inglese, tema chiaro/scuro, accessibile (ruoli ARIA, annunci di stato, focus visibile, tastiera, movimento ridotto).
- Diagnostica `--self-test`.

## 3. Test

31 test automatici superati (6 interfaccia, 11 unità Rust, 1 IPC Tauri,
13 integrazione con 7-Zip reale); la suite Rust passa anche compilata x86_64.
0 avvisi clippy. Dettagli ed elenco di ciò che non è stato verificato in
[TEST.md](TEST.md).

## 4. Licenze

- Codice originale: **MIT** © 2026 Alessandro Pezzali.
- 7-Zip 26.05: **LGPL 2.1+ con restrizione unRAR, BSD 3/2-clause**; eseguibile
  ufficiale non modificato, processo separato; `License.txt` incluso; mai dichiarato MIT.
- 388 crate Rust e 6 pacchetti npm con licenze permissive; testi completi in
  `THIRD_PARTY_NOTICES.md` (incluso nei pacchetti). Nessuna GPL/AGPL.
- Analisi completa in [AUDIT.md](AUDIT.md).

## 5. Piattaforme

| Piattaforma | Build | Test | Avvio |
|---|---|---|---|
| macOS arm64 (26.6.2) | ✅ `.app`, `.dmg` | ✅ 31/31 | ✅ |
| macOS universale (arm64 + x86_64) | ✅ `.app`, `.dmg` | ✅ suite Rust x86_64 con Rosetta | ✅ `--self-test` x86_64 con Rosetta |
| Windows x64/arm64 | non eseguita | non eseguiti | — |
| Linux x64/arm64 | non eseguita | non eseguiti | — |

## 6. Limitazioni note

1. **Windows e Linux non verificati.** Punti da controllare: lettura della
   password da stdin con caratteri non ASCII su Windows, percorsi del motore
   incluso nei pacchetti `.deb`/AppImage, estrazione dell'installer 7-Zip con
   `7zr.exe` nello script `fetch-7zip` su host Windows.
2. **Nessuna firma a pagamento**: macOS mostra l'avviso di Gatekeeper (firma
   ad-hoc, nessuna notarizzazione); Windows mostrerà SmartScreen.
3. Password ZIP limitate ai caratteri ASCII (limite di 7-Zip): l'app lo spiega e suggerisce 7z.
4. Su macOS "Apri con…" dal Finder non passa ancora il file all'app (serve la
   gestione degli eventi Apple); funzionano trascinamento e scelta file.
5. Non è possibile creare RAR (licenza unRAR); RAR/ISO/CAB reali non testati per mancanza di campioni.
6. Nessuna associazione dei tipi di file e nessuna estrazione selettiva di singoli file (possibili sviluppi).
7. Interazione con la finestra nativa non automatizzata (catture schermo bloccate dai permessi macOS, non modificati); l'interfaccia è stata verificata in un browser headless con IPC simulato.

## 7. Preparazione alla pubblicazione

**Pronta per una pubblicazione macOS di prova**, previa tua autorizzazione.
Prima di pubblicare:

1. Decidere se verificare Windows e Linux (es. su un PC/VM esistente, oppure
   avviando Colima già installato: richiede la tua autorizzazione perché
   scrive fuori dal progetto).
2. Allegare alla release il sorgente `7z2605-src.tar.xz` (obbligo LGPL).
3. Indicare nelle note le piattaforme realmente verificate.
4. Push e release: **non eseguiti**, in attesa di autorizzazione.

Commit locali: presenti sul ramo `main`. Nessun push, deploy o release effettuati.
Nessun costo sostenuto.
