// Componenti condivisi: avanzamento, errori, risultato, password, area di rilascio.
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { Issue, OpError } from "../api";
import { hasKey, useI18n, type Key } from "../i18n";
import { passwordStrength } from "../format";

export interface Busy {
  jobId: string;
  phase: string;
  percent: number;
  cancelling: boolean;
}

export function ProgressPanel({ busy, onCancel }: { busy: Busy; onCancel: () => void }) {
  const { t } = useI18n();
  const phaseKey = `phase.${busy.cancelling ? "cancelling" : busy.phase}`;
  const label = hasKey(phaseKey) ? t(phaseKey) : busy.phase;
  const indeterminate = busy.phase === "analyze" || busy.phase === "list" || busy.phase === "verify";
  return (
    <div className="card progress" role="status" aria-live="polite">
      <div className="progress-head">
        <span className="spinner" aria-hidden="true" />
        <strong>{label}</strong>
        {!indeterminate && <span className="progress-pct">{busy.percent}%</span>}
      </div>
      <div
        className={`bar ${indeterminate ? "indeterminate" : ""}`}
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={indeterminate ? undefined : busy.percent}
      >
        <div className="bar-fill" style={{ width: indeterminate ? undefined : `${busy.percent}%` }} />
      </div>
      <button className="btn ghost" onClick={onCancel} disabled={busy.cancelling}>
        {t("action.cancel")}
      </button>
    </div>
  );
}

/** Porta in vista un avviso appena compare (può trovarsi sotto la piega). */
export function useRevealOnMount<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  useEffect(() => {
    ref.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, []);
  return ref;
}

export function ErrorBox({ error, children }: { error: OpError; children?: ReactNode }) {
  const { t } = useI18n();
  const ref = useRevealOnMount<HTMLDivElement>();
  const key = `err.${error.code}`;
  const text = hasKey(key) ? t(key as Key) : error.message;
  const details = error.details.filter(Boolean);
  return (
    <div ref={ref} className={`card alert ${error.code === "cancelled" ? "info" : "danger"}`} role="alert">
      <strong>{error.code === "cancelled" ? t("action.cancel") : t("err.title")}</strong>
      <p>{text}</p>
      {details.length > 0 && (
        <details>
          <summary>{t("err.details")}</summary>
          <pre>{details.join("\n")}</pre>
        </details>
      )}
      {children}
    </div>
  );
}

export function IssueList({ issues }: { issues: Issue[] }) {
  const { t } = useI18n();
  const shown = issues.slice(0, 50);
  return (
    <ul className="issues">
      {shown.map((i, n) => {
        const key = `issue.${i.kind}`;
        return (
          <li key={n}>
            <span className="issue-kind">{hasKey(key) ? t(key) : i.kind}</span>
            {i.path && <code>{i.path}</code>}
            {i.detail && <span className="muted"> {i.detail}</span>}
          </li>
        );
      })}
      {issues.length > shown.length && <li className="muted">{t("archive.more", { n: issues.length - shown.length })}</li>}
    </ul>
  );
}

export function ResultCard(props: {
  title: string;
  path?: string;
  detail?: string;
  warnings?: Issue[];
  onAgain: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="card success" role="status" aria-live="polite">
      <div className="result-head">
        <span className="check" aria-hidden="true">✓</span>
        <div>
          <h2>{props.title}</h2>
          {props.detail && <p className="muted">{props.detail}</p>}
        </div>
      </div>
      {props.path && <code className="path">{props.path}</code>}
      {props.warnings && props.warnings.length > 0 && (
        <div className="alert warn inner">
          <strong>{t("done.warnings")}</strong>
          <IssueList issues={props.warnings} />
        </div>
      )}
      <div className="actions">
        {props.path && (
          <button className="btn primary" onClick={() => revealItemInDir(props.path!).catch(() => {})}>
            {t("action.reveal")}
          </button>
        )}
        <button className="btn" onClick={props.onAgain}>
          {t("action.again")}
        </button>
      </div>
    </div>
  );
}

export function PasswordField(props: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  showStrength?: boolean;
  autoFocus?: boolean;
  disabled?: boolean;
  placeholder?: string;
  onEnter?: () => void;
}) {
  const { t } = useI18n();
  const id = useId();
  const [visible, setVisible] = useState(false);
  const strength = passwordStrength(props.value);
  return (
    <div className="field">
      <label htmlFor={id}>{props.label}</label>
      <div className="pw-row">
        <input
          id={id}
          type={visible ? "text" : "password"}
          value={props.value}
          autoFocus={props.autoFocus}
          disabled={props.disabled}
          placeholder={props.placeholder}
          autoComplete="off"
          spellCheck={false}
          onChange={(e) => props.onChange(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && props.onEnter?.()}
        />
        <button
          type="button"
          className="btn ghost small"
          onClick={() => setVisible((v) => !v)}
          aria-pressed={visible}
          disabled={props.disabled}
        >
          {visible ? t("password.hide") : t("password.show")}
        </button>
      </div>
      {props.showStrength && props.value && (
        <div className="strength" aria-live="polite">
          <div className={`meter s${strength}`} aria-hidden="true">
            <span /><span /><span /><span />
          </div>
          <span className="muted">
            {t("password.strength")}: {t(`password.strength${strength}` as Key)}
          </span>
        </div>
      )}
    </div>
  );
}

export function DropZone(props: {
  title: string;
  hint: string;
  dragging: boolean;
  children: ReactNode;
  icon: "archive" | "files";
}) {
  const { t } = useI18n();
  return (
    <div className={`dropzone ${props.dragging ? "over" : ""}`}>
      <div className="drop-icon" aria-hidden="true">
        {props.icon === "archive" ? <ArchiveIcon /> : <FilesIcon />}
      </div>
      <h2>{props.dragging ? t("drop.release") : props.title}</h2>
      <p className="muted">{props.hint}</p>
      <div className="actions center">{props.children}</div>
    </div>
  );
}

function ArchiveIcon() {
  return (
    <svg viewBox="0 0 64 64" width="56" height="56">
      <rect x="10" y="12" width="44" height="42" rx="8" fill="var(--surface-2)" stroke="var(--brand)" strokeWidth="3" />
      <rect x="10" y="12" width="44" height="11" rx="6" fill="var(--accent)" />
      <path d="M32 12v18" stroke="var(--brand)" strokeWidth="4" strokeDasharray="3 3" />
      <path d="M32 34v12m-6-6 6 6 6-6" stroke="var(--teal)" strokeWidth="4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function FilesIcon() {
  return (
    <svg viewBox="0 0 64 64" width="56" height="56">
      <rect x="14" y="8" width="30" height="38" rx="5" fill="var(--surface-2)" stroke="var(--brand)" strokeWidth="3" />
      <rect x="22" y="16" width="30" height="38" rx="5" fill="var(--surface)" stroke="var(--teal)" strokeWidth="3" />
      <path d="M37 42V28m-6 6 6-6 6 6" stroke="var(--teal)" strokeWidth="4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
