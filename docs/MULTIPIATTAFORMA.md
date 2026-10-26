# Verifica multipiattaforma — strategia a costo zero

Stato al 26 ottobre 2026: **verificati macOS e Linux (Ubuntu 24.04 in
container, arm64 nativo e amd64 tramite Rosetta)**; Windows non verificato.
Esiti in [TEST.md](TEST.md). Questo documento descrive come verificare le
piattaforme senza servizi a pagamento, carta di credito o fatturazione a consumo.
Il punto 1 è stato eseguito con autorizzazione; il punto 2 richiede il push,
non ancora autorizzato.

## 1. Linux in locale con Colima (raccomandato per primo)

### Fattibilità (rilevata il 25 ottobre 2026, prima dell'avvio)

| Elemento | Stato |
|---|---|
| Colima 0.10.3, Lima 2.2.0, Docker CLI 29.8.1 | installati (Homebrew) |
| Profilo `default` esistente | `vmType: vz`, `arch: aarch64`, 8 CPU, 16 GB RAM, disco 80 GB, `rosetta: true`, `mountType: virtiofs` |
| Disco della VM già allocato | ~11 GB in `~/.colima` |
| Rosetta 2 sul Mac | installata → container `linux/amd64` eseguibili tramite Rosetta |
| Spazio libero sul Mac | 119 GiB (disco all'87%) |
| VM in esecuzione | nessuna |

Licenze e costi: Colima e Lima sono gratuiti e open source (MIT, Apache 2.0),
Ubuntu e i pacchetti Debian sono gratuiti, l'immagine `ubuntu:24.04` si
scarica da Docker Hub senza account. **Costo: zero.**

Effetti fuori dal progetto (motivo per cui serve l'autorizzazione):
- `colima start` avvia la VM e scrive in `~/.colima` (disco della VM, log, socket);
- le immagini Docker (stima: 3–4 GB con le dipendenze) restano nel disco della VM;
- servono download una tantum (immagine, pacchetti apt, Node.js, Rust, crate,
  7-Zip con SHA-256 verificato); dopo, la build non dipende dalla rete
  tranne `npm ci` e `cargo` senza cache.

### Procedura preparata

File nel repository: `ci/linux/Dockerfile`, `scripts/linux-docker.sh`.

```bash
colima start                       # da eseguire solo dopo autorizzazione
scripts/linux-docker.sh arm64      # Linux arm64 nativo
scripts/linux-docker.sh amd64      # Linux x86_64 tramite Rosetta
# prova del pacchetto installato e dell'interfaccia (esempio arm64):
docker run --rm --platform linux/arm64 \
  -v "$PWD/linux-out/arm64:/pkg:ro" -v "$PWD/ci/linux/test-package.sh:/opt/test.sh:ro" \
  -v "$PWD/linux-out/arm64/ui:/out" \
  ubuntu@sha256:18b33ff3829707f31ada654a963b9ee35f2e6f0241b964bd17a22a9b330cb63d bash /opt/test.sh
colima stop                        # al termine
```

Lo script monta il repository **in sola lettura**, compila una copia del commit
corrente dentro il container ed esegue: `npm ci`, `fetch-7zip`, `tsc`, Vitest,
`cargo test` (unità, IPC, integrazione con 7-Zip reale), `clippy`, build
`.deb`, `--self-test` con display virtuale (Xvfb). Unica scrittura sul Mac:
`linux-out/<arch>/` (pacchetti, log, SHA-256), ignorata da git.

Lancio dei comandi lunghi: avviare Colima e i job in una sessione di processo
separata (es. `nohup`/`setsid`). Durante la verifica la VM è stata arrestata
una volta da un segnale di interruzione destinato al terminale da cui era
stata avviata; nessun dato perso.

### Perché solo `.deb` e non AppImage

L'AppImage è stato generato in prova e poi **escluso** dalla distribuzione:
1. `linuxdeploy` modifica gli eseguibili inclusi (imposta `RUNPATH $ORIGIN`):
   il `7zz` dentro l'AppImage non era più identico all'eseguibile ufficiale;
2. include circa 150 librerie di sistema (GTK, WebKitGTK, GStreamer, GnuTLS…,
   molte LGPL) senza i relativi testi di licenza né offerta dei sorgenti;
3. `linuxdeploy` e il suo plugin GTK vengono scaricati da GitHub senza
   verifica d'integrità, uno dei due da un tag mobile ("continuous").

Il `.deb` invece contiene solo i file del progetto e il `7zz` ufficiale non
modificato, e lascia le librerie di sistema al gestore pacchetti della
distribuzione. Un AppImage conforme richiederebbe strumenti verificati e un
inventario completo delle licenze delle librerie incluse.

Limiti: verifica build, test automatici e avvio dell'app con display virtuale;
non sostituisce una prova manuale su un desktop Linux reale (GNOME/KDE,
trascinamento file, dialoghi nativi). amd64 tramite Rosetta è un'emulazione.

## 2. Windows e Linux con GitHub Actions (repository pubblico)

Il repository `pezzaliapp/PezzaliAPPZipUnzip` risulta **pubblico** e ancora
vuoto. Per i repository pubblici GitHub dichiara gratuiti i runner standard
(documentazione "About billing for GitHub Actions", ribadito negli annunci
sui prezzi 2026). I runner "larger" sono a pagamento anche nei repository
pubblici e **non vengono usati**.

Preparato: `.github/workflows/ci.yml`:
- matrice: `ubuntu-24.04`, `ubuntu-24.04-arm`, `windows-latest`, `windows-11-arm`, `macos-latest`;
- passi: dipendenze, `fetch-7zip`, `tsc`, Vitest, `cargo test`, `clippy`,
  `tauri build`, `--self-test`;
- sicurezza: permessi `contents: read`, nessun segreto, nessuna release,
  credenziali di checkout non conservate; pacchetti come artefatti
  temporanei (7 giorni).

Garanzie di costo zero da controllare prima del push:
1. il repository resta **pubblico**;
2. nessun metodo di pagamento sull'account GitHub (Settings → Billing): senza
   metodo di pagamento GitHub non può addebitare nulla;
3. nella matrice solo etichette standard. Le etichette arm64 sono indicate da
   GitHub come gratuite per i repository pubblici: va riconfermato sulla
   pagina "Choosing the runner for a job" il giorno del primo push, e tolte
   dalla matrice in caso di dubbio.

Richiede il **push** (non autorizzato finora). È l'unica via gratuita trovata
per eseguire i test su Windows reale senza procurarsi una licenza Windows.

## 3. Windows in locale (alternativa, non raccomandata ora)

- **PC Windows già posseduto**: `npm install`, `npm run fetch-7zip`, `npm test`,
  `cargo test`, `npm run tauri build` (vedi [BUILD.md](BUILD.md)). Costo zero.
- **VM Windows 11 ARM su questo Mac**: richiede di installare un
  virtualizzatore (es. UTM, gratuito) e una ISO di Windows 11 ARM64 da
  Microsoft. Installare nuove applicazioni è fuori dall'autonomia concessa e
  l'uso di Windows richiede una licenza valida: **va valutato da te**
  (aspetto legale), per questo non è la strada proposta.
- **Compilazione incrociata da macOS** (`cargo-xwin`): produce l'eseguibile ma
  non permette di eseguire i test, e scarica componenti Microsoft con una loro
  licenza. Non sostituisce la verifica.

## Ordine consigliato

1. ~~Linux arm64 e amd64 con Colima~~ — eseguito il 25-26 ottobre 2026.
2. Dopo l'autorizzazione al push: GitHub Actions per Windows x64/arm64 (e
   conferma di Linux x86_64 e macOS su macchine reali pulite).
3. Aggiornare `docs/TEST.md` e il rapporto finale con i soli esiti ottenuti.
