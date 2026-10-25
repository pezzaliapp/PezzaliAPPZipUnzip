import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, newJobId, toOpError, type Engine, type OpError, type PathInfo } from "./api";
import { detectLang, I18n, makeT, type Lang } from "./i18n";
import { isArchiveName } from "./format";
import { ErrorBox, ProgressPanel, type Busy } from "./components/common";
import { ExtractView } from "./components/ExtractView";
import { CompressView } from "./components/CompressView";
import { AboutDialog } from "./components/AboutDialog";

export type RunJob = <T>(phase: string, fn: (jobId: string) => Promise<T>) => Promise<T>;
type Tab = "extract" | "compress";

export default function App() {
  const [lang, setLang] = useState<Lang>(detectLang);
  const t = useMemo(() => makeT(lang), [lang]);
  const [tab, setTab] = useState<Tab>("extract");
  const [engine, setEngine] = useState<Engine | null>(null);
  const [engineError, setEngineError] = useState<OpError | null>(null);
  const [busy, setBusy] = useState<Busy | null>(null);
  const busyRef = useRef<Busy | null>(null);
  const [dragging, setDragging] = useState(false);
  const [archive, setArchive] = useState<string | null>(null);
  const [sources, setSources] = useState<PathInfo[]>([]);
  const [aboutOpen, setAboutOpen] = useState(false);
  const tabRef = useRef<Tab>(tab);
  tabRef.current = tab;

  useEffect(() => {
    document.documentElement.lang = lang;
    try {
      localStorage.setItem("lang", lang);
    } catch {
      /* ignorato */
    }
  }, [lang]);

  useEffect(() => {
    api.engineInfo().then(setEngine, (e) => setEngineError(toOpError(e)));
  }, []);

  const updateBusy = (b: Busy | null) => {
    busyRef.current = b;
    setBusy(b);
  };

  useEffect(() => {
    const un = api.onProgress((p) => {
      const cur = busyRef.current;
      if (cur && cur.jobId === p.jobId) updateBusy({ ...cur, phase: p.phase, percent: p.percent });
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const runJob: RunJob = useCallback(async (phase, fn) => {
    const jobId = newJobId();
    updateBusy({ jobId, phase, percent: 0, cancelling: false });
    try {
      return await fn(jobId);
    } finally {
      updateBusy(null);
    }
  }, []);

  const cancel = () => {
    const cur = busyRef.current;
    if (!cur) return;
    updateBusy({ ...cur, cancelling: true });
    void api.cancel(cur.jobId);
  };

  const addPaths = useCallback(async (paths: string[]) => {
    const infos = await api.pathInfo(paths);
    setSources((prev) => {
      const known = new Set(prev.map((p) => p.path));
      return [...prev, ...infos.filter((i) => !known.has(i.path))];
    });
  }, []);

  const handlePaths = useCallback(
    (paths: string[]) => {
      if (paths.length === 0 || busyRef.current) return;
      if (tabRef.current === "extract") {
        const arch = paths.find(isArchiveName) ?? paths[0];
        setArchive(arch);
      } else {
        void addPaths(paths);
      }
    },
    [addPaths],
  );

  // Trascinamento di file nella finestra (percorsi reali forniti da Tauri).
  useEffect(() => {
    const un = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") setDragging(true);
      else if (p.type === "leave") setDragging(false);
      else if (p.type === "drop") {
        setDragging(false);
        handlePaths(p.paths);
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, [handlePaths]);

  // File passati all'avvio ("Apri con").
  useEffect(() => {
    api.launchPaths().then((paths) => {
      if (paths.length === 0) return;
      if (paths.length === 1 && isArchiveName(paths[0])) setArchive(paths[0]);
      else {
        setTab("compress");
        void addPaths(paths);
      }
    }, () => {});
  }, [addPaths]);

  const onTabKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      const next = tab === "extract" ? "compress" : "extract";
      setTab(next);
      document.getElementById(`tab-${next}`)?.focus();
    }
  };

  const engineLabel = engine
    ? t(engine.bundled ? "engine.bundled" : "engine.system", { v: engine.version })
    : engineError
      ? t("engine.missing")
      : t("engine.loading");

  return (
    <I18n.Provider value={{ t, lang }}>
      <div className="app">
        <header className="topbar">
          <div className="brand">
            <img src="/icon.svg" alt="" width="36" height="36" />
            <div>
              <h1>
                PezzaliAPP<span>ZipUnzip</span>
              </h1>
              <p className="muted">{t("app.tagline")}</p>
            </div>
          </div>
          <div className="top-actions">
            <span className={`chip ${engineError ? "bad" : ""}`} title={engine?.path}>{engineLabel}</span>
            <span className="chip ok">{t("offline")}</span>
            <button className="btn ghost small" onClick={() => setLang(lang === "it" ? "en" : "it")}>
              {t("lang.switch")}
            </button>
            <button className="icon-btn" aria-label={t("about.open")} title={t("about.open")} onClick={() => setAboutOpen(true)}>
              i
            </button>
          </div>
        </header>

        <nav className="tabs" role="tablist" aria-label="PezzaliAPPZipUnzip" onKeyDown={onTabKey}>
          {(["extract", "compress"] as const).map((id) => (
            <button
              key={id}
              id={`tab-${id}`}
              role="tab"
              aria-selected={tab === id}
              aria-controls={`panel-${id}`}
              tabIndex={tab === id ? 0 : -1}
              className={tab === id ? "on" : ""}
              onClick={() => setTab(id)}
              disabled={!!busy}
            >
              {t(id === "extract" ? "tab.extract" : "tab.compress")}
            </button>
          ))}
        </nav>

        <main id={`panel-${tab}`} role="tabpanel" aria-labelledby={`tab-${tab}`}>
          {engineError ? (
            <ErrorBox error={engineError}>
              <p>{t("engine.missingHelp")}</p>
            </ErrorBox>
          ) : tab === "extract" ? (
            <ExtractView archive={archive} setArchive={setArchive} dragging={dragging} busy={!!busy} runJob={runJob} />
          ) : (
            <CompressView
              sources={sources}
              setSources={setSources}
              addPaths={addPaths}
              dragging={dragging}
              busy={!!busy}
              runJob={runJob}
            />
          )}
          {busy && <ProgressPanel busy={busy} onCancel={cancel} />}
        </main>

        <footer className="footer muted">© 2026 Alessandro Pezzali · pezzaliAPP · MIT · 7-Zip © Igor Pavlov (LGPL)</footer>
        {aboutOpen && <AboutDialog engine={engine} onClose={() => setAboutOpen(false)} />}
      </div>
    </I18n.Provider>
  );
}
