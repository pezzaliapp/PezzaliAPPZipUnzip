import { useEffect, useRef } from "react";
import type { Engine } from "../api";
import { useI18n } from "../i18n";

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
      <p>{t("about.engine")}</p>
      <p>{t("about.privacy")}</p>
      <p>{t("about.safety")}</p>
      <p className="muted">{t("about.notices")}</p>
      {engine && (
        <p className="muted">
          {t("about.enginePath")}: <code>{engine.path}</code> (7-Zip {engine.version})
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
