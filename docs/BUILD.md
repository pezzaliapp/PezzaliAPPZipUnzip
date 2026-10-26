# Build e pubblicazione

Tutti gli strumenti sono gratuiti. Nessuna firma a pagamento è richiesta.

## Prerequisiti comuni

- Node.js 22.12+ o 24 e npm (richiesti da Vite 8 e Vitest 5)
- Rust stabile (`rustup`), 1.77 o successivo
- Prerequisiti di Tauri 2 per il sistema: https://v2.tauri.app/start/prerequisites/

```bash
npm install
npm run fetch-7zip   # motore 7-Zip 26.05 per la piattaforma corrente (SHA-256 verificato)
```

`fetch-7zip` accetta `--target mac|linux-x64|linux-arm64|win-x64|win-arm64`.
Il motore finisce in `src-tauri/engine/`, cartella non versionata. In sviluppo,
senza il motore incluso, l'app cerca 7-Zip installato nel sistema (`7zz`, `7z`, `7za`).

`npm run tauri build` passa da `scripts/tauri.mjs`, che:
- si ferma se il motore manca, così nessun pacchetto viene distribuito senza 7-Zip;
- aggiunge `--remap-path-prefix` per la cartella utente e il progetto, così i
  binari non contengono percorsi locali né il nome dell'utente.

## macOS (verificato)

```bash
npm run tauri build                                   # architettura corrente
npm run tauri build -- --target universal-apple-darwin  # Apple Silicon + Intel
```

Risultati in `src-tauri/target/<target>/release/bundle/` (`.app`, `.dmg`).
L'app ha firma *ad-hoc* e non è notarizzata (servirebbe l'Apple Developer
Program, a pagamento). Al primo avvio: clic destro sull'app → **Apri**, oppure
Impostazioni di Sistema → Privacy e sicurezza → **Apri comunque**.

Strategia gratuita per verificare Windows e Linux: [MULTIPIATTAFORMA.md](MULTIPIATTAFORMA.md).

## Windows (verificato su Windows Server 2025 x64 con GitHub Actions)

Su Windows 10/11 con Microsoft C++ Build Tools e WebView2 (già presente in
Windows 11):

```powershell
npm install
npm run fetch-7zip     # scarica 7z2605-x64.exe e 7zr.exe, estrae 7z.exe + 7z.dll senza eseguire l'installer
npm run tauri build    # produce installer MSI e NSIS
```

Senza firma Authenticode (a pagamento) SmartScreen mostrerà un avviso:
"Ulteriori informazioni" → "Esegui comunque".

## Linux (verificato su Ubuntu 24.04 in container)

Dipendenze (Debian/Ubuntu): `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`.

```bash
npm install
npm run fetch-7zip
npm run tauri build -- --bundles deb    # solo .deb: AppImage escluso, vedi MULTIPIATTAFORMA.md
```

## Verifica di un pacchetto

```bash
<eseguibile> --self-test
# PezzaliAPPZipUnzip 1.0.0 — 7-Zip 26.05 (incluso) …/engine/7zz
```

## Avvisi di terze parti

Dopo aver aggiornato le dipendenze o il motore:

```bash
node scripts/third-party.mjs
```

## Lista di controllo per la pubblicazione (richiede autorizzazione)

1. `npm test` e `cargo test` superati sulla piattaforma di build.
2. `npm run fetch-7zip` sulla piattaforma di destinazione; `--self-test` riuscito.
3. `node scripts/third-party.mjs` rigenerato e incluso.
4. **Allegare alla release il sorgente di 7-Zip** `7z2605-src.tar.xz`
   (https://github.com/ip7z/7zip/releases/tag/26.05, SHA-256
   `0b6080b8c3e7a579161d2d495224c0e51c467576cb4c7048b0ab485ef44cc22e`) — obbligo LGPL.
5. Indicare nelle note di rilascio le piattaforme effettivamente verificate.
6. Push e pubblicazione **solo previa autorizzazione** di Alessandro Pezzali.
