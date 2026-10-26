# PezzaliAPPZipUnzip

**Comprimi ed estrai archivi. Offline, sicuro, gratuito.**
App desktop dell'ecosistema **pezzaliAPP** — autore unico **Alessandro Pezzali**.

PezzaliAPPZipUnzip è un'interfaccia moderna e accessibile per il motore open
source ufficiale [7-Zip](https://www.7-zip.org). Funziona interamente sul tuo
computer: nessuna connessione di rete, nessuna telemetria, nessun account.

## Funzioni

- **Estrai** 7z, ZIP, RAR, TAR, GZ, BZ2, XZ, ZST, ISO, CAB e gli altri formati
  letti da 7-Zip, compresi i tarball compressi (`.tar.gz`, `.tar.xz`, …) in un
  solo passaggio.
- **Comprimi** file e cartelle in **7z**, **ZIP**, **TAR**, **TAR.GZ**, **TAR.XZ**,
  con cinque livelli di compressione.
- **Password**: cifratura AES-256 per 7z (anche dei nomi dei file) e ZIP;
  sblocco degli archivi protetti. La password non compare mai nella riga di
  comando dei processi.
- **Anteprima del contenuto** e **verifica dell'integrità** prima di estrarre.
- **Trascina e rilascia** file e archivi nella finestra.
- **Annullamento** in qualsiasi momento, senza lasciare file parziali.
- Interfaccia in **italiano** e **inglese**, tema **chiaro** e **scuro**,
  navigazione da tastiera, supporto ai lettori di schermo, movimento ridotto.

## Protezione di file e privacy

- Analisi di sicurezza **prima** dell'estrazione: blocco di Zip Slip (`../`),
  percorsi assoluti, link simbolici e hard link verso l'esterno, rapporti di
  compressione anomali (decompression bomb).
- Estrazione in una **cartella temporanea** nella destinazione, poi verifica:
  rimozione dei link pericolosi, dei permessi setuid/setgid, separazione degli
  hard link esterni; spostamento finale con un **nome libero**.
- **Nessuna sovrascrittura**: se `Foto` esiste già, il risultato diventa `Foto (2)`.
- **Gli originali non vengono mai modificati né eliminati.**
- **I contenuti estratti non vengono mai eseguiti.** Su macOS i file estratti
  ereditano l'attributo di quarantena dell'archivio scaricato.
- Controllo continuo dello spazio su disco e della dimensione reale estratta.

Dettagli in [docs/SICUREZZA.md](docs/SICUREZZA.md).

## Piattaforme

| Piattaforma | Stato |
|---|---|
| macOS 26 Apple Silicon | ✅ verificata: build, test completi, avvio dell'app |
| macOS Intel (x86_64) | ⚠️ build universale; test eseguiti tramite Rosetta 2, non su hardware Intel |
| Windows x64 | ✅ test automatici, installer e `--self-test` dopo l'installazione su Windows Server 2025 (GitHub Actions); interfaccia non provata a mano |
| Linux arm64 | ✅ verificata in container Ubuntu 24.04: build, test completi, pacchetto `.deb`, interfaccia su schermo virtuale |
| Linux x64 | ✅ test automatici su hardware x86_64 (GitHub Actions); `.deb` e interfaccia provati in container con emulazione Rosetta |

Vedi [docs/TEST.md](docs/TEST.md) per gli esiti reali.

## Compilare dai sorgenti

Requisiti: Node.js 20+, Rust 1.77+, prerequisiti di sistema di
[Tauri 2](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run fetch-7zip        # scarica 7-Zip 26.05 ufficiale e ne verifica lo SHA-256
npm run tauri dev         # avvio in sviluppo
npm run tauri build       # pacchetto per la piattaforma corrente
```

Test:

```bash
npm test                              # interfaccia (Vitest)
cd src-tauri && cargo test            # Rust: unità, IPC, integrazione con 7-Zip reale
```

Istruzioni complete per ogni sistema in [docs/BUILD.md](docs/BUILD.md).

## Licenze

- Codice originale di PezzaliAPPZipUnzip: **MIT** — © 2026 Alessandro Pezzali ([LICENSE](LICENSE)).
- Motore **7-Zip** © Igor Pavlov: programma separato, **non** coperto dalla
  licenza MIT. GNU LGPL 2.1+ con **restrizione unRAR**, parti BSD 3-clause e
  2-clause. Il codice unRAR non può essere usato per ricreare l'algoritmo di
  compressione RAR: l'app estrae archivi RAR ma non li crea.
- Dipendenze Rust e JavaScript: licenze permissive, elencate con i testi
  completi in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Costi

Nessuno. L'app non usa servizi online, API, abbonamenti o server. Strumenti di
sviluppo e dipendenze sono gratuiti e open source.

---

### English

PezzaliAPPZipUnzip is a free, offline desktop app (Tauri 2 + React + Rust) to
compress and extract archives with the official open-source 7-Zip engine.
No network, no telemetry. Safe extraction (Zip Slip, dangerous links and
decompression bomb protection, no overwrites, originals never deleted).
App code: MIT © 2026 Alessandro Pezzali. 7-Zip: LGPL 2.1 + unRAR restriction + BSD.
