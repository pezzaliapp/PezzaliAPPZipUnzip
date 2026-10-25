import { useEffect, useId, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, toOpError, type CompressResult, type Format, type OpError, type PathInfo } from "../api";
import { dirName, formatBytes, isAscii, isValidName, suggestArchiveName } from "../format";
import { useI18n, type Key } from "../i18n";
import { DropZone, ErrorBox, PasswordField, ResultCard } from "./common";
import type { RunJob } from "../App";

const FORMATS: { id: Format; label: string; hint: Key }[] = [
  { id: "7z", label: "7z", hint: "compress.hint7z" },
  { id: "zip", label: "ZIP", hint: "compress.hintZip" },
  { id: "tar", label: "TAR", hint: "compress.hintTar" },
  { id: "tar-gz", label: "TAR.GZ", hint: "compress.hintTarGz" },
  { id: "tar-xz", label: "TAR.XZ", hint: "compress.hintTarXz" },
];

const LEVELS = [0, 1, 5, 7, 9] as const;

export function CompressView(props: {
  sources: PathInfo[];
  addPaths: (paths: string[]) => void;
  setSources: (s: PathInfo[]) => void;
  dragging: boolean;
  busy: boolean;
  runJob: RunJob;
}) {
  const { t, lang } = useI18n();
  const { sources, setSources, addPaths, runJob, busy } = props;
  const [name, setName] = useState("");
  const [nameTouched, setNameTouched] = useState(false);
  const [format, setFormat] = useState<Format>("zip");
  const [level, setLevel] = useState<number>(5);
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [encryptNames, setEncryptNames] = useState(true);
  const [excludeSystem, setExcludeSystem] = useState(true);
  const [outDir, setOutDir] = useState("");
  const [outTouched, setOutTouched] = useState(false);
  const [error, setError] = useState<OpError | null>(null);
  const [result, setResult] = useState<CompressResult | null>(null);
  const nameId = useId();
  const levelId = useId();

  useEffect(() => {
    if (!nameTouched) setName(suggestArchiveName(sources.map((s) => s.path)));
    if (!outTouched && sources.length > 0) setOutDir(dirName(sources[0].path));
  }, [sources, nameTouched, outTouched]);

  const supportsPassword = format === "7z" || format === "zip";
  const pw = supportsPassword ? password : "";
  const mismatch = pw.length > 0 && confirm !== pw;
  const zipAsciiProblem = format === "zip" && pw.length > 0 && !isAscii(pw);
  const nameOk = isValidName(name);
  const total = sources.reduce((s, x) => s + x.size, 0);

  const pickFiles = async () => {
    const picked = await open({ multiple: true, directory: false });
    if (Array.isArray(picked)) addPaths(picked);
    else if (typeof picked === "string") addPaths([picked]);
  };
  const pickFolder = async () => {
    const picked = await open({ multiple: true, directory: true });
    if (Array.isArray(picked)) addPaths(picked);
    else if (typeof picked === "string") addPaths([picked]);
  };
  const chooseOut = async () => {
    const picked = await open({ directory: true, multiple: false, defaultPath: outDir || undefined });
    if (typeof picked === "string") {
      setOutDir(picked);
      setOutTouched(true);
    }
  };

  const doCompress = async () => {
    setError(null);
    try {
      const r = await runJob("compress", (id) =>
        api.compress(id, sources.map((s) => s.path), outDir, name.trim(), {
          format,
          level: format === "tar" ? 0 : level,
          password: pw || null,
          encryptNames: format === "7z" && encryptNames,
          excludeSystemFiles: excludeSystem,
        }),
      );
      setResult(r);
    } catch (e) {
      setError(toOpError(e));
    }
  };

  const startOver = () => {
    setResult(null);
    setSources([]);
    setPassword("");
    setConfirm("");
    setNameTouched(false);
    setOutTouched(false);
  };

  if (result) {
    return (
      <ResultCard
        title={t("done.compress")}
        path={result.output}
        detail={formatBytes(result.size, lang)}
        onAgain={startOver}
      />
    );
  }

  if (sources.length === 0) {
    return (
      <DropZone icon="files" title={t("drop.files")} hint={t("drop.filesHint")} dragging={props.dragging}>
        <button className="btn primary" onClick={pickFiles}>{t("pick.files")}</button>
        <button className="btn" onClick={pickFolder}>{t("pick.folder")}</button>
      </DropZone>
    );
  }

  const canRun = !busy && nameOk && !!outDir && !mismatch && !zipAsciiProblem;

  return (
    <div className="stack">
      <section className={`card ${props.dragging ? "over" : ""}`} aria-labelledby="src-title">
        <div className="file-head">
          <div className="grow">
            <h2 id="src-title">
              {sources.length === 1
                ? t("compress.item1", { size: formatBytes(total, lang) })
                : t("compress.items", { n: sources.length, size: formatBytes(total, lang) })}
            </h2>
          </div>
          <button className="btn ghost small" onClick={pickFiles} disabled={busy}>{t("pick.files")}</button>
          <button className="btn ghost small" onClick={pickFolder} disabled={busy}>{t("pick.folder")}</button>
        </div>
        <ul className="sources">
          {sources.map((s) => (
            <li key={s.path}>
              <span aria-hidden="true">{s.isDir ? "📁" : "📄"}</span>
              <span className="grow truncate" title={s.path}>{s.name}</span>
              <span className="muted">{formatBytes(s.size, lang)}</span>
              <button
                className="icon-btn"
                aria-label={t("action.remove", { name: s.name })}
                onClick={() => setSources(sources.filter((x) => x.path !== s.path))}
                disabled={busy}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      </section>

      <section className="card grid2">
        <div className="field span2">
          <label htmlFor={nameId}>{t("compress.name")}</label>
          <input
            id={nameId}
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              setNameTouched(true);
            }}
            aria-invalid={!nameOk}
            spellCheck={false}
          />
          {!nameOk && <span className="error-text">{t("compress.invalidName")}</span>}
        </div>

        <fieldset className="field span2">
          <legend>{t("compress.format")}</legend>
          <div className="segmented" role="radiogroup">
            {FORMATS.map((f) => (
              <label key={f.id} className={format === f.id ? "on" : ""}>
                <input
                  type="radio"
                  name="format"
                  value={f.id}
                  checked={format === f.id}
                  onChange={() => setFormat(f.id)}
                />
                {f.label}
              </label>
            ))}
          </div>
          <span className="muted">{t(FORMATS.find((f) => f.id === format)!.hint)}</span>
        </fieldset>

        {format !== "tar" && (
          <div className="field">
            <label htmlFor={levelId}>{t("compress.level")}</label>
            <select id={levelId} value={level} onChange={(e) => setLevel(Number(e.target.value))}>
              {LEVELS.map((l) => (
                <option key={l} value={l}>{t(`compress.level${l}` as Key)}</option>
              ))}
            </select>
          </div>
        )}

        <div className="field">
          <span className="label">{t("dest.compress")}</span>
          <div className="dest-row">
            <code className="path grow truncate" title={outDir}>{outDir}</code>
            <button className="btn small" onClick={chooseOut} disabled={busy}>{t("pick.dest")}</button>
          </div>
        </div>

        {supportsPassword ? (
          <>
            <PasswordField
              label={`${t("password.label")} (${t("password.optional").toLowerCase()})`}
              value={password}
              onChange={setPassword}
              showStrength
              disabled={busy}
            />
            {password && (
              <PasswordField label={t("password.confirm")} value={confirm} onChange={setConfirm} disabled={busy} />
            )}
            {mismatch && confirm && <p className="error-text span2" role="alert">{t("password.mismatch")}</p>}
            {zipAsciiProblem && <p className="error-text span2" role="alert">{t("password.zipAscii")}</p>}
            {format === "7z" && password && (
              <label className="check span2">
                <input type="checkbox" checked={encryptNames} onChange={(e) => setEncryptNames(e.target.checked)} />
                {t("compress.encryptNames")}
              </label>
            )}
          </>
        ) : (
          <p className="muted span2">{t("password.unsupported")}</p>
        )}

        <label className="check span2">
          <input type="checkbox" checked={excludeSystem} onChange={(e) => setExcludeSystem(e.target.checked)} />
          {t("compress.excludeSystem")}
        </label>

        <div className="actions span2">
          <button className="btn primary big" onClick={doCompress} disabled={!canRun}>
            {t("action.compress")}
          </button>
          <button className="btn ghost" onClick={() => setSources([])} disabled={busy}>
            {t("action.clear")}
          </button>
        </div>
      </section>

      {error && <ErrorBox error={error} />}
    </div>
  );
}
