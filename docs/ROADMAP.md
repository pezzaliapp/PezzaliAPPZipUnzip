# Roadmap

Miglioramenti proposti dopo la 1.0.0, in ordine di priorità. Vincoli invariati:
costo zero, offline, nessuna telemetria, nessun servizio esterno obbligatorio,
nessuna dipendenza nuova se non indispensabile. Ogni voce va realizzata solo
dopo approvazione, con test e aggiornamento di [TEST.md](TEST.md).

## Priorità alta — correttezza e verifica

1. **Verifica Windows completata**: chiudere gli eventuali errori del CI
   Windows x64 (`.github/workflows/ci.yml`); in particolare la password da
   stdin con caratteri non ASCII e la lettura di nomi di file Unicode.
2. **Prova su desktop Linux reale** (GNOME o KDE): trascinamento dal file
   manager, dialoghi nativi di scelta file, "Apri con". Oggi Linux è verificato
   solo in container con schermo virtuale.
3. **"Apri con…" su macOS**: gestire l'evento di apertura file inviato dal
   Finder (oggi funzionano solo trascinamento e scelta file). Nessuna nuova
   dipendenza: evento `RunEvent::Opened` di Tauri.
4. **Finestra Informazioni** troppo alta a 680 px: ridurre spaziature o
   raggruppare le attribuzioni in una sezione apribile.

## Usabilità

- Trascinamento: indicare già durante il trascinamento se il file è un
  archivio da estrarre o un elemento da comprimere; al rilascio di più
  archivi nella scheda Estrai (oggi viene usato il primo, in silenzio)
  avvisare quali sono stati ignorati.
- Anteprima: ricerca per nome nell'elenco dei file (oggi mostra le prime 300
  voci).
- Ricordare (solo in locale) l'ultima cartella di destinazione e il formato di
  compressione scelto.
- Scorciatoie da tastiera documentate nella finestra Informazioni.

## Gestione degli errori

- Messaggi distinti per: password errata, archivio danneggiato, volume
  multiparte mancante, spazio insufficiente, permessi negati, percorso troppo
  lungo su Windows; per ognuno un'azione suggerita.
- "Copia dettagli" negli errori: testo tecnico (codice d'uscita di 7-Zip,
  messaggio) da incollare in una segnalazione, senza percorsi personali.

## Sicurezza nell'estrazione

- Limiti contro le decompression bomb oggi fissi (64 GiB, rapporto 1000:1
  sopra 1 GiB, 512 MiB liberi): renderli visibili e modificabili entro valori
  sicuri, per chi lavora con archivi molto grandi.
- Archivi annidati (archivio dentro archivio): avviso esplicito, nessuna
  estrazione automatica ricorsiva.
- Test con campioni reali RAR, ISO e CAB creati con strumenti liberi o
  scaricati da fonti ufficiali con licenza compatibile.
- Windows: verifica dei nomi riservati (`CON`, `NUL`, …) e dei flussi
  alternativi NTFS nell'analisi preventiva, con test dedicati.

## Prestazioni con archivi grandi

- Elenco dei file: rendering a finestra scorrevole per archivi con decine di
  migliaia di voci, senza nuove librerie.
- Misurare tempi e memoria con archivi da 1, 10 e 50 GB e con 100 000 file;
  registrare i risultati in TEST.md.
- Verifica successiva all'estrazione: valutare un controllo più leggero per
  archivi molto grandi, mantenendo la protezione da sovrascritture.

## Compatibilità multipiattaforma

- Windows arm64 e Linux arm64 su runner standard gratuiti (dopo aver
  riconfermato la gratuità sulla documentazione di GitHub).
- Altre distribuzioni Linux (Fedora, Debian stabile) in container locali.
- AppImage solo se si trova una procedura con strumenti verificati e un
  inventario completo delle licenze delle librerie incluse
  (vedi [MULTIPIATTAFORMA.md](MULTIPIATTAFORMA.md)).

## Aggiornamenti futuri senza servizi a pagamento

- Distribuzione tramite le **release di GitHub** (gratuite per i repository
  pubblici), con `SHA256SUMS` e il sorgente di 7-Zip allegato (obbligo LGPL).
- Nessun aggiornamento automatico in rete: l'app resta offline. Al massimo un
  link "Controlla aggiornamenti" che apre la pagina delle release nel browser,
  solo su richiesta dell'utente.
- Aggiornamento di 7-Zip: modificare versione e SHA-256 in
  `scripts/fetch-7zip.mjs`, rieseguire tutti i test e aggiornare
  `THIRD_PARTY_NOTICES.md`.
- Firma del codice: Gatekeeper e SmartScreen mostrano un avviso senza firma a
  pagamento. Restare senza firma a pagamento e documentare come aprire l'app;
  valutare solo opzioni di firma gratuite, se disponibili.
