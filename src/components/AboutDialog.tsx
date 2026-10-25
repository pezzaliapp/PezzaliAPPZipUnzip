import { useEffect, useRef } from "react";
import type { Engine } from "../api";
import { useI18n } from "../i18n";

/** Versione distribuita da scripts/fetch-7zip.mjs (mostrata se il motore non risponde). */
const BUNDLED_7ZIP = "26.05";
const SEVEN_ZIP_SITE = "https://www.7-zip.org/";

export function AboutDialog({ engine, onClose }: { engine: Engine | null; onClose: () => void }) {
  const { t } = useI18n();
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const d = ref.current;
    if (d && !d.open) d.showModal();
  }, []);

  return (
    <dialog ref={ref} className="dialog" aria-labelledby="about-title" onClose={onClose} onCancel={onClose}>
      <h2 id="about-title">PezzaliAPPZipUnzip 1.0.0</h2>
      <p>{t("about.author")}</p>
      <p>{t("about.license")}</p>
      <p>{t("about.privacy")}</p>
      <p>{t("about.safety")}</p>

      <h3>{t("about.engineTitle", { v: engine?.version ?? BUNDLED_7ZIP })}</h3>
      <p>{t("about.engineSeparate")}</p>
      <p>{t("about.engineCopyright")}</p>
      <p>{t("about.engineLicenses")}</p>
      {/* Testo selezionabile, non un link: l'app non apre pagine web. */}
      <p>
        {t("about.engineSite")} <code>{SEVEN_ZIP_SITE}</code>
      </p>
      <p className="muted">{t("about.notices")}</p>
      {engine && (
        <p className="muted">
          {t("about.enginePath")}: <code>{engine.path}</code>
        </p>
      )}
      <div className="actions">
        <button className="btn primary" onClick={() => ref.current?.close()} autoFocus>
          {t("close")}
        </button>
      </div>
    </dialog>
  );
}
