import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, toOpError, type ExtractResult, type Listing, type OpError, type TestResult } from "../api";
import { ARCHIVE_EXTENSIONS, baseName, dirName, formatBytes, formatLabel } from "../format";
import { useI18n } from "../i18n";
import { DropZone, ErrorBox, IssueList, PasswordField, ResultCard, useRevealOnMount } from "./common";
import type { RunJob } from "../App";

const PREVIEW_LIMIT = 300;

export function ExtractView(props: {
  archive: string | null;
  setArchive: (p: string | null) => void;
  dragging: boolean;
  busy: boolean;
  runJob: RunJob;
}) {
  const { t, lang } = useI18n();
  const { archive, setArchive, runJob, busy } = props;
  const [listing, setListing] = useState<Listing | null>(null);
  const [needPassword, setNeedPassword] = useState(false);
  const [password, setPassword] = useState("");
  const [dest, setDest] = useState("");
  const [error, setError] = useState<OpError | null>(null);
  const [result, setResult] = useState<ExtractResult | null>(null);
  const [tested, setTested] = useState<TestResult | null>(null);

  const reset = () => {
    setListing(null);
    setNeedPassword(false);
    setPassword("");
    setError(null);
    setResult(null);
    setTested(null);
  };

  const readListing = async (path: string, pw: string | null) => {
    setError(null);
    try {
      const l = await runJob("list", (id) => api.list(id, path, pw));
      setListing(l);
      setNeedPassword(l.encrypted);
    } catch (e) {
      const err = toOpError(e);
      if (err.code === "need-password") {
        setNeedPassword(true);
      } else {
        setError(err);
      }
    }
  };

  useEffect(() => {
    reset();
    if (archive) {
      setDest(dirName(archive));
      void readListing(archive, null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [archive]);

  const stats = listing && { files: listing.totalFiles, dirs: listing.totalDirs, size: listing.totalSize };

  const chooseArchive = async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: t("tab.extract"), extensions: ARCHIVE_EXTENSIONS }, { name: "*", extensions: ["*"] }],
    });
    if (typeof picked === "string") setArchive(picked);
  };

  const chooseDest = async () => {
    const picked = await open({ directory: true, multiple: false, defaultPath: dest || undefined });
    if (typeof picked === "string") setDest(picked);
  };

  const pw = needPassword && password ? password : null;

  const doExtract = async (allowUnsafe: boolean) => {
    if (!archive) return;
    setError(null);
    try {
      const r = await runJob("analyze", (id) => api.extract(id, archive, dest, pw, allowUnsafe));
      setResult(r);
    } catch (e) {
      const err = toOpError(e);
      if (err.code === "need-password") setNeedPassword(true);
      setError(err);
    }
  };

  const doTest = async () => {
    if (!archive) return;
    setError(null);
    setTested(null);
    try {
      setTested(await runJob("test", (id) => api.test(id, archive, pw)));
    } catch (e) {
      setError(toOpError(e));
    }
  };

  const unlock = () => archive && password && readListing(archive, password);

  if (!archive) {
    return (
      <DropZone icon="archive" title={t("drop.archive")} hint={t("drop.archiveHint")} dragging={props.dragging}>
        <button className="btn primary" onClick={chooseArchive}>
          {t("pick.archive")}
        </button>
      </DropZone>
    );
  }

  if (result) {
    return (
      <ResultCard
        title={t("done.extract")}
        path={result.output}
        detail={t("done.files", { files: result.files, size: formatBytes(result.bytes, lang) })}
        warnings={result.warnings}
        onAgain={() => setArchive(null)}
      />
    );
  }

  const unsafe = error?.code === "unsafe";
  const locked = needPassword && !listing;

  return (
    <div className="stack">
      <section className="card" aria-labelledby="archive-title">
        <div className="file-head">
          <div className="file-badge" aria-hidden="true">{listing ? formatLabel(listing.format).slice(0, 6) : "…"}</div>
          <div className="grow">
            <h2 id="archive-title" className="truncate" title={archive}>{baseName(archive)}</h2>
            <p className="muted truncate">{dirName(archive)}</p>
          </div>
          <button className="btn ghost small" onClick={() => setArchive(null)} disabled={busy}>
            {t("archive.change")}
          </button>
        </div>

        {listing && stats && (
          <dl className="facts">
            <div><dt>{t("archive.format")}</dt><dd>{formatLabel(listing.format)}</dd></div>
            <div><dt>{t("archive.size")}</dt><dd>{formatBytes(stats.size, lang)}</dd></div>
            <div>
              <dt>{t("archive.content")}</dt>
              <dd>
                {stats.files === 1 ? t("archive.file1") : t("archive.files", { n: stats.files })}
                {" · "}
                {stats.dirs === 1 ? t("archive.folder1") : t("archive.folders", { n: stats.dirs })}
              </dd>
            </div>
            {needPassword && <div><dt>🔒</dt><dd>{t("archive.encrypted")}</dd></div>}
          </dl>
        )}

        {needPassword && (
          <div className="password-box">
            {locked && <p>{t("password.needed")}</p>}
            <PasswordField
              label={t("password.label")}
              value={password}
              onChange={setPassword}
              autoFocus
              disabled={busy}
              onEnter={locked ? unlock : undefined}
            />
            {locked && (
              <button className="btn primary" onClick={unlock} disabled={!password || busy}>
                {t("password.unlock")}
              </button>
            )}
          </div>
        )}

        {listing && listing.entries.length > 0 && (
          <details className="preview">
            <summary>{t("archive.preview")}</summary>
            <ul>
              {listing.entries.slice(0, PREVIEW_LIMIT).map((e, i) => (
                <li key={i} className={e.isDir ? "dir" : ""}>
                  <span className="truncate">{e.isDir ? "📁 " : ""}{e.path || baseName(archive)}</span>
                  {!e.isDir && <span className="muted">{formatBytes(e.size, lang)}</span>}
                </li>
              ))}
              {listing.totalFiles + listing.totalDirs > PREVIEW_LIMIT && (
                <li className="muted">
                  {t("archive.more", { n: listing.totalFiles + listing.totalDirs - PREVIEW_LIMIT })}
                </li>
              )}
            </ul>
          </details>
        )}
      </section>

      {!locked && (
        <section className="card">
          <div className="field">
            <span className="label">{t("dest.label")}</span>
            <div className="dest-row">
              <code className="path grow truncate" title={dest}>{dest}</code>
              <button className="btn small" onClick={chooseDest} disabled={busy}>{t("pick.dest")}</button>
            </div>
          </div>
          <div className="actions">
            <button
              className="btn primary big"
              onClick={() => doExtract(false)}
              disabled={busy || !dest || (needPassword && !password)}
            >
              {t("action.extract")}
            </button>
            <button className="btn" onClick={doTest} disabled={busy || (needPassword && !password)}>
              {t("action.test")}
            </button>
          </div>
        </section>
      )}

      {tested && (
        <div className="card success" role="status">
          <strong>✓ {t("done.test")}</strong>
          <p className="muted">{t("done.testDetail", { n: tested.files })}</p>
        </div>
      )}

      {unsafe && error && (
        <UnsafeWarning error={error} busy={busy} onBack={() => setError(null)} onProceed={() => doExtract(true)} />
      )}
      {error && !unsafe && <ErrorBox error={error} />}
    </div>
  );
}

function UnsafeWarning(props: { error: OpError; busy: boolean; onBack: () => void; onProceed: () => void }) {
  const { t } = useI18n();
  const ref = useRevealOnMount<HTMLDivElement>();
  return (
    <div ref={ref} className="card alert warn" role="alert">
      <strong>{t("unsafe.title")}</strong>
      <p>{t("unsafe.body")}</p>
      <IssueList issues={props.error.issues} />
      <p>{t("unsafe.protected")}</p>
      <div className="actions">
        <button className="btn" onClick={props.onBack}>{t("action.back")}</button>
        <button className="btn danger" onClick={props.onProceed} disabled={props.busy}>
          {t("action.extractAnyway")}
        </button>
      </div>
    </div>
  );
}
