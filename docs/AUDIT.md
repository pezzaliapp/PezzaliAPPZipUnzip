# Audit tecnico, economico e legale

Data: 25 ottobre 2026 · Progetto: PezzaliAPPZipUnzip · Autore: Alessandro Pezzali

## 1. Ambiente di sviluppo (verificato)

| Strumento | Versione | Note |
|---|---|---|
| macOS | 26.6.2 (arm64) | macchina di sviluppo |
| Xcode / Command Line Tools | presenti | necessari per la build macOS |
| Node.js / npm | 24.19.0 / 11.17.0 | già installati (nvm) |
| Rust / Cargo | 1.96.0 | target `aarch64-apple-darwin`, `x86_64-apple-darwin` già installati |
| Tauri CLI | 2.11.3 globale preesistente; 2.12.4 locale (`@tauri-apps/cli`, devDependency) | nessuna installazione globale effettuata |
| 7-Zip | 26.01 (Homebrew, preesistente); 26.05 ufficiale scaricato nel progetto | |
| Docker/Colima | installati ma non avviati | non usati: avviarli modifica file fuori dal progetto |

Nessuno strumento globale è stato installato, nessun privilegio amministrativo
richiesto, nessuna configurazione di sistema modificata.

## 2. Audit economico

| Voce | Esito |
|---|---|
| Servizi online, API, cloud, server | **Nessuno.** L'app è offline. |
| Dipendenze | Tutte open source e gratuite (npm, crates.io). |
| Motore 7-Zip | Gratuito, anche per uso commerciale (License.txt). |
| Download di build | Release pubbliche gratuite (GitHub, 7-zip.org). |
| Firma del codice | **Non effettuata.** Apple Developer Program (99 USD/anno) e certificati Windows sono a pagamento: esclusi. Su macOS si usa la firma *ad-hoc* gratuita. |
| CI/CD | Nessuna pipeline attivata (eviterebbe qualsiasi rischio di consumo minuti). |
| Costi futuri | Nessuno previsto. |

**Esito: nessun costo, presente o futuro.**

## 3. Audit legale

### 3.1 7-Zip

- Licenza ufficiale (da `License.txt` della release 26.05): GNU LGPL 2.1 o
  successiva per la maggior parte del codice; LGPL con **restrizione unRAR** per
  il decompressore RAR; BSD 3-clause (LZFSE, ZSTD); BSD 2-clause (XXH64).
- Modalità d'uso scelta: **eseguibile ufficiale non modificato, avviato come
  processo separato**. Nessun collegamento o incorporamento di codice 7-Zip:
  l'app non diventa opera derivata e può restare MIT.
- Obblighi rispettati:
  - `License.txt` distribuito accanto all'eseguibile (`engine/License.txt`) e
    riportato integralmente in `THIRD_PARTY_NOTICES.md`;
  - nessuna dichiarazione MIT per 7-Zip (vedi `LICENSE`, README, finestra
    "Informazioni");
  - restrizione unRAR dichiarata nella documentazione; l'app non crea archivi RAR;
  - sorgente corrispondente indicato con URL e SHA-256
    (`7z2605-src.tar.xz`). **Alla pubblicazione** va allegato alla release, nello
    stesso luogo dei binari (LGPL 2.1 §4/§6).
- Integrità: hash SHA-256 dei binari verificati su due fonti indipendenti
  (download diretto e digest della release GitHub `ip7z/7zip` 26.05).
- Nessun reverse engineering: si usa solo l'interfaccia a riga di comando documentata.

### 3.2 Tauri, React e dipendenze

- Tauri 2, React 19: MIT / Apache-2.0.
- 388 crate Rust per le piattaforme desktop: MIT, Apache-2.0, BSD, Zlib,
  Unicode-3.0, Unlicense, CC0, MPL-2.0 (usate senza modifiche). Nessuna
  licenza GPL/AGPL nel codice collegato.
- Testi di licenza raccolti automaticamente da `scripts/third-party.mjs`.

### 3.3 Marchi e identità

- Nome, icona e interfaccia sono originali dell'ecosistema pezzaliAPP.
- "7-Zip" è citato solo come nome del motore, con attribuzione all'autore.

## 4. Idoneità della tecnologia

| Requisito | Scelta | Motivazione |
|---|---|---|
| Desktop Windows/macOS/Linux | Tauri 2 | binari piccoli (≈14 MB app universale), webview di sistema |
| Interfaccia moderna | React 19 + TypeScript | componenti accessibili, tipizzazione |
| Logica sicura | Rust | gestione processi, file system e controlli di sicurezza |
| Motore | 7-Zip CLI ufficiale | formati più ampi, licenza compatibile se eseguito separatamente |
| Offline | CSP restrittiva, nessuna richiesta di rete | |

**Esito dell'audit: nessun impedimento economico, legale o tecnico. Si procede.**
