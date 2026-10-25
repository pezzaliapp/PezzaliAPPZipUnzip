// Funzioni pure di supporto all'interfaccia (testate con Vitest).

export const ARCHIVE_EXTENSIONS = [
  "7z", "zip", "rar", "tar", "gz", "tgz", "bz2", "tbz", "tbz2", "xz", "txz", "zst", "tzst",
  "lzma", "z", "cab", "iso", "arj", "cpio", "rpm", "deb", "wim", "lzh", "lha", "dmg", "jar",
  "apk", "001",
];

export function isArchiveName(name: string): boolean {
  const lower = name.toLowerCase();
  return ARCHIVE_EXTENSIONS.some((ext) => lower.endsWith(`.${ext}`));
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (i <= 0) return i === 0 ? path.slice(0, 1) : "";
  // "C:\file" → "C:\"
  if (i === 2 && path[1] === ":") return path.slice(0, 3);
  return path.slice(0, i);
}

const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function formatBytes(bytes: number, locale = "it"): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  let v = bytes;
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u += 1;
  }
  const digits = u === 0 ? 0 : v < 10 ? 1 : 0;
  return `${v.toLocaleString(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits })} ${UNITS[u]}`;
}

/** Nome suggerito per un nuovo archivio a partire dai file scelti. */
export function suggestArchiveName(paths: string[]): string {
  if (paths.length === 0) return "Archivio";
  if (paths.length === 1) {
    const name = baseName(paths[0]);
    const dot = name.lastIndexOf(".");
    return dot > 0 ? name.slice(0, dot) : name;
  }
  return baseName(dirName(paths[0])) || "Archivio";
}

export function formatLabel(format: string): string {
  return format.toUpperCase().replace("-", ".");
}

/** Valutazione semplice della robustezza della password (0–4). */
export function passwordStrength(pw: string): number {
  if (!pw) return 0;
  let score = 0;
  if (pw.length >= 8) score++;
  if (pw.length >= 14) score++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) score++;
  if (/\d/.test(pw) && /[^A-Za-z0-9]/.test(pw)) score++;
  return Math.min(score, 4);
}

export function isAscii(s: string): boolean {
  return /^[\x20-\x7e]*$/.test(s);
}

export const VALID_NAME = /^[^/\\:*?"<>|\0]+$/;

export function isValidName(name: string): boolean {
  const n = name.trim();
  return n.length > 0 && n !== "." && n !== ".." && VALID_NAME.test(n) && n.length <= 255;
}
