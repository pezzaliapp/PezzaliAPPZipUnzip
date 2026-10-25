// Ponte tipizzato verso i comandi Rust (src-tauri/src/lib.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Engine {
  path: string;
  version: string;
  bundled: boolean;
}

export interface Entry {
  path: string;
  size: number;
  packed: number;
  modified: string;
  isDir: boolean;
  encrypted: boolean;
  symlink: string | null;
  hardlink: string | null;
  linkUnknownTarget: boolean;
}

export interface Listing {
  format: string;
  physicalSize: number;
  headersEncrypted: boolean;
  /** Al massimo 2000 voci: i totali sotto valgono per l'intero archivio. */
  entries: Entry[];
  totalFiles: number;
  totalDirs: number;
  totalSize: number;
  encrypted: boolean;
  truncated: boolean;
}

export interface Issue {
  kind: string;
  path: string;
  detail: string;
}

export interface OpError {
  code: string;
  message: string;
  details: string[];
  issues: Issue[];
}

export interface ExtractResult {
  output: string;
  files: number;
  dirs: number;
  bytes: number;
  warnings: Issue[];
}

export interface CompressResult {
  output: string;
  size: number;
}

export interface TestResult {
  ok: boolean;
  format: string;
  files: number;
}

export interface PathInfo {
  path: string;
  name: string;
  isDir: boolean;
  size: number;
}

export type Format = "7z" | "zip" | "tar" | "tar-gz" | "tar-xz";

export interface CompressOptions {
  format: Format;
  level: number;
  password: string | null;
  encryptNames: boolean;
  excludeSystemFiles: boolean;
}

export interface Progress {
  jobId: string;
  phase: string;
  percent: number;
}

export function isOpError(e: unknown): e is OpError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export function toOpError(e: unknown): OpError {
  if (isOpError(e)) return { ...e, details: e.details ?? [], issues: e.issues ?? [] };
  return { code: "internal", message: String(e), details: [], issues: [] };
}

let counter = 0;
export function newJobId(): string {
  counter += 1;
  return `job-${Date.now().toString(36)}-${counter}`;
}

export const api = {
  engineInfo: () => invoke<Engine>("engine_info"),
  list: (jobId: string, archive: string, password: string | null) =>
    invoke<Listing>("list_archive", { jobId, archive, password }),
  extract: (jobId: string, archive: string, destDir: string, password: string | null, allowUnsafe: boolean) =>
    invoke<ExtractResult>("extract_archive", { jobId, archive, destDir, password, allowUnsafe }),
  compress: (jobId: string, sources: string[], outDir: string, name: string, options: CompressOptions) =>
    invoke<CompressResult>("compress_files", { jobId, sources, outDir, name, options }),
  test: (jobId: string, archive: string, password: string | null) =>
    invoke<TestResult>("test_archive", { jobId, archive, password }),
  cancel: (jobId: string) => invoke<void>("cancel_job", { jobId }),
  pathInfo: (paths: string[]) => invoke<PathInfo[]>("path_info", { paths }),
  launchPaths: () => invoke<string[]>("launch_paths"),
  onProgress: (cb: (p: Progress) => void): Promise<UnlistenFn> =>
    listen<Progress>("job-progress", (e) => cb(e.payload)),
};
