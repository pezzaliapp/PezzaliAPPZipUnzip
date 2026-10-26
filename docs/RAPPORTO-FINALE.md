# Rapporto finale — PezzaliAPPZipUnzip 1.0.0

Data: 25 ottobre 2026, aggiornato il 26 ottobre 2026 · Autore: Alessandro Pezzali · Ecosistema pezzaliAPP

## 1. Esito

App desktop completa e funzionante per comprimere ed estrarre archivi, basata
sul motore ufficiale 7-Zip 26.05, interamente offline e senza costi.
**Verificata su macOS** (Apple Silicon, nativo; Intel tramite Rosetta).
**Verificata su Linux** (Ubuntu 24.04 in container: arm64 nativo, x86_64
tramite emulazione Rosetta; pacchetto `.deb`, interfaccia su schermo virtuale).
**Windows è supportato dal codice ma non ancora verificato.**

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
| Linux arm64 (Ubuntu 24.04, container nativo) | ✅ `.deb` | ✅ 6 + 12 + 13, clippy 0 | ✅ `.deb` installato, `--self-test`, interfaccia su Xvfb |
| Linux x86_64 (Ubuntu 24.04, container con Rosetta) | ✅ `.deb` | ✅ 6 + 12 + 13, clippy 0 (emulazione) | ✅ idem (emulazione) |
| Linux AppImage | escluso (licenze e binario 7-Zip modificato, vedi MULTIPIATTAFORMA.md) | — | — |

## 6. Limitazioni note

1. **Windows non verificato.** Punti da controllare: lettura della
   password da stdin con caratteri non ASCII, estrazione dell'installer 7-Zip
   con `7zr.exe` nello script `fetch-7zip` su host Windows. Linux verificato
   solo in container Ubuntu 24.04 (non su un desktop reale né su hardware
   x86_64 reale).
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

1. Decidere se verificare Windows: strategia gratuita pronta in
   [MULTIPIATTAFORMA.md](MULTIPIATTAFORMA.md) (GitHub Actions su repository
   pubblico, richiede l'autorizzazione al push).
2. Allegare alla release il sorgente `7z2605-src.tar.xz` (obbligo LGPL).
3. Indicare nelle note le piattaforme realmente verificate.
4. Push e release: **non eseguiti**, in attesa di autorizzazione.

Ultimo pacchetto macOS (25 ottobre 2026, interfaccia senza badge del motore,
nessun percorso personale nei binari):
`src-tauri/target/universal-apple-darwin/release/bundle/dmg/PezzaliAPPZipUnzip_1.0.0_universal.dmg`,
SHA-256 `234a513e5f1f10d19015b3dd1a379ca06777c1dc24e137fd8d4e6d171b43043b`.

Pacchetti Linux (26 ottobre 2026), in `linux-out/` (ignorata da git):
- `linux-out/arm64/PezzaliAPPZipUnzip_1.0.0_arm64.deb`, SHA-256 `a5945265d20fca42115461b6c5f8b14f326e99225ea7a6bea20a36cdd6560aa3`
- `linux-out/amd64/PezzaliAPPZipUnzip_1.0.0_amd64.deb`, SHA-256 `22324310aaf416ed6ffddccb73c56e735e7b09283194c792b3df4027aa54ce58`

Commit locali: presenti sul ramo `main`. Nessun push, deploy o release effettuati.
Nessun costo sostenuto.
