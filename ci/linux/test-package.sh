#!/bin/bash
# Prova del pacchetto .deb in Ubuntu pulito: installazione, --self-test dal
# percorso installato, avvio dell'interfaccia su schermo virtuale (Xvfb) con
# screenshot della finestra principale e della finestra Informazioni.
# Uso (dal Mac, con Colima avviato): vedi docs/MULTIPIATTAFORMA.md
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive LANG=C.UTF-8
apt-get update -qq
apt-get install -y -qq --no-install-recommends /pkg/*.deb xvfb xauth xdotool imagemagick x11-utils fonts-dejavu-core >/dev/null
echo "== installato: $(dpkg -s pezzali-app-zip-unzip | grep -E '^(Status|Version)' | tr '\n' ' ')"
echo "== self-test (pacchetto installato)"
xvfb-run -a pezzaliapp-zipunzip --self-test 2>/dev/null
# Interfaccia: tutto dentro xvfb-run (display virtuale con server X dedicato).
cat > /tmp/ui.sh <<'UI'
set -euo pipefail
cd /tmp
pezzaliapp-zipunzip >/out/app.log 2>&1 &
APP=$!
W=""
for i in $(seq 1 30); do W=$(xdotool search --name "PezzaliAPPZipUnzip" 2>/dev/null | head -1 || true); [ -n "$W" ] && break; sleep 1; done
echo "== finestra: ${W:-non trovata}"; [ -n "$W" ]
xwininfo -id "$W" | grep -E "Width|Height"
sleep 4
import -window root /out/linux-home.png
eval "$(xdotool getwindowgeometry --shell "$W")"
# pulsante Informazioni (in alto a destra)
xdotool mousemove --window "$W" $((WIDTH-60)) 44 click 1; sleep 2
import -window root /out/linux-about.png
kill -0 $APP && echo "== app ancora in esecuzione: ok"
kill $APP; wait $APP 2>/dev/null || true
UI
xvfb-run -a -s "-screen 0 1100x900x24" bash /tmp/ui.sh
grep -iE "error|panic" /out/app.log | grep -v "libEGL\|DRI3" || echo "== nessun errore nel log dell'app"
