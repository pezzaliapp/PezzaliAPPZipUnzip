# Sicurezza

## Minacce considerate e contromisure

| Minaccia | Contromisura | Test |
|---|---|---|
| **Zip Slip** (`../../file`) | blocco preventivo (`path-traversal`); 7-Zip normalizza comunque i percorsi; estrazione in cartella temporanea | `zip_slip_is_blocked`, `safety::detects_traversal_and_absolute` |
| Percorsi assoluti (`/etc/x`, `C:\…`) | blocco preventivo (`absolute-path`) | idem |
| **Symlink verso l'esterno** | blocco preventivo; dopo l'estrazione rimozione dei link con bersaglio assoluto o esterno (controllo lessicale + `canonicalize`) | `symlink_attacks_are_blocked`, `safety::post_scan_neutralizes` |
| Hard link verso file esterni | blocco preventivo; dopo l'estrazione i file con collegamenti esterni vengono sostituiti da copie | idem |
| **Decompression bomb** | rapporto > 1000:1 sopra 1 GB segnalato; limite 64 GB; spazio libero verificato prima; watchdog sulla dimensione reale e sullo spazio durante l'estrazione; controllo finale | `bomb_ratio_is_detected`, `forged_size_is_stopped`, `watchdog_kills_engine` |
| Dimensioni dichiarate false | 7-Zip rifiuta i dati oltre la dimensione dichiarata (verificato con ZIP e GZIP falsificati); watchdog e controllo finale come seconda linea | `forged_size_is_stopped` |
| **Sovrascrittura** di file esistenti | destinazione sempre con nome libero (`nome (2)`); `-aou` contro le collisioni interne | `never_overwrites`, `safety::unique_never_overwrites` |
| **Perdita di dati** in caso di errore | lavoro in cartella temporanea; nessun file parziale lasciato; originali mai toccati (nessun `-sdel`) | `cancel_cleans_up`, `passwords_7z_and_zip`, `roundtrip_all_formats` (hash degli originali) |
| Esecuzione di contenuti estratti | l'app non apre né esegue i file; "Mostra nella cartella" usa solo `reveal`; bit setuid/setgid rimossi; quarantena macOS propagata | `post_scan_neutralizes` |
| Password esposte | password scritte su stdin, mai negli argomenti dei processi; nessun salvataggio | test password |
| Iniezione di comandi | nessuna shell; argomenti separati; `--` prima dei nomi di file | file `-trattino.txt` nei test |
| Nomi di archivio malevoli | validazione (niente separatori, `..`, caratteri riservati) | `safety::names` |
| Interfaccia web compromessa | CSP restrittiva (nessuna risorsa remota), `freezePrototype`, permessi minimi (`dialog:allow-open`, `opener:allow-reveal-item-in-dir`) | configurazione |
| Motore manomesso | binari ufficiali con SHA-256 verificato al download; il motore incluso ha priorità su quello di sistema | `scripts/fetch-7zip.mjs` |

## Privacy

- Nessuna richiesta di rete: la CSP consente solo l'IPC locale di Tauri.
- Nessuna telemetria, nessun log inviato, nessun account.
- Unica preferenza salvata: la lingua (localStorage della webview).
- In compressione, i file di sistema (`.DS_Store`, `._*`, `Thumbs.db`,
  `desktop.ini`) sono esclusi per impostazione predefinita.

## Limiti noti

- Il controllo del rapporto di compressione si basa sulle dimensioni dichiarate
  nell'archivio; le dimensioni false sono gestite da 7-Zip e dal watchdog.
- Archivi annidati (ZIP dentro ZIP) non vengono estratti ricorsivamente, quindi
  non possono moltiplicare la dimensione.
- Su Windows i nomi riservati (`CON`, `NUL`, …) sono gestiti da 7-Zip; non
  verificato su Windows reale.
- In "modo protetto" (dopo l'avviso) l'estrazione prosegue: 7-Zip ignora i link
  pericolosi e la verifica successiva rimuove quelli residui.
