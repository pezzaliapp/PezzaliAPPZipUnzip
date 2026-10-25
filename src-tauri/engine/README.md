# Motore 7-Zip incluso nel pacchetto

Questa cartella viene popolata da `npm run fetch-7zip`, che scarica la release
ufficiale di 7-Zip (https://www.7-zip.org, https://github.com/ip7z/7zip),
ne verifica l'hash SHA-256 e copia qui l'eseguibile per la piattaforma corrente
insieme a `License.txt`.

I binari non sono versionati nel repository. Se la cartella è vuota l'app usa
un 7-Zip già installato nel sistema (`7zz`, `7z`, `7za`).
