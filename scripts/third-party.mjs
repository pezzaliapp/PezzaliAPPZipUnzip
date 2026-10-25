#!/usr/bin/env node
// Genera THIRD_PARTY_NOTICES.md con le licenze di 7-Zip e di tutte le
// dipendenze Rust e npm incluse nell'app (testi letti dai pacchetti reali).
//
// Uso: node scripts/third-party.mjs

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const LICENSE_FILE = /^(LICEN[SC]E|COPYING|NOTICE|UNLICENSE)([-._].*)?$/i;

function licenseTexts(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => LICENSE_FILE.test(f))
    .sort()
    .map((f) => ({ file: f, text: readFileSync(join(dir, f), "utf8").trim() }));
}

// ---- Rust: dipendenze normali e di build raggiungibili dall'app,
// unione delle piattaforme desktop distribuite.
const DESKTOP_TARGETS = [
  "aarch64-apple-darwin", "x86_64-apple-darwin",
  "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc",
  "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu",
];
const pkgs = new Map();
const seen = new Set();
for (const target of DESKTOP_TARGETS) {
  const meta = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--filter-platform", target], {
      cwd: join(root, "src-tauri"),
      maxBuffer: 256 * 1024 * 1024,
    }).toString(),
  );
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  for (const p of meta.packages) pkgs.set(p.id, p);
  const visited = new Set();
  const stack = [meta.resolve.root];
  while (stack.length) {
    const id = stack.pop();
    if (visited.has(id)) continue;
    visited.add(id);
    for (const d of nodes.get(id).deps) {
      if (d.dep_kinds.some((k) => k.kind === null || k.kind === "build")) stack.push(d.pkg);
    }
  }
  visited.delete(meta.resolve.root);
  for (const id of visited) seen.add(id);
}
const crates = [...seen]
  .map((id) => pkgs.get(id))
  .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));

// ---- npm: dipendenze di produzione (incluse nel bundle dell'interfaccia).
const npmTree = JSON.parse(
  execFileSync("npm", ["ls", "--omit=dev", "--all", "--json"], { cwd: root, shell: process.platform === "win32" }).toString(),
);
const npmPkgs = new Map();
(function walk(deps) {
  for (const [name, info] of Object.entries(deps ?? {})) {
    const dir = join(root, "node_modules", name);
    if (!npmPkgs.has(name) && existsSync(join(dir, "package.json"))) {
      const pj = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
      npmPkgs.set(name, { name, version: pj.version, license: pj.license ?? "?", repository: pj.repository?.url ?? pj.repository ?? "", dir });
    }
    walk(info.dependencies);
  }
})(npmTree.dependencies);
const npm = [...npmPkgs.values()].sort((a, b) => a.name.localeCompare(b.name));

// ---- Testi di licenza raggruppati quando identici.
const groups = new Map();
function addTexts(label, dir) {
  for (const { text } of licenseTexts(dir)) {
    if (!groups.has(text)) groups.set(text, []);
    groups.get(text).push(label);
  }
}
for (const c of crates) addTexts(`${c.name} ${c.version}`, dirname(c.manifest_path));
for (const p of npm) addTexts(`${p.name} ${p.version}`, p.dir);

const missing = [
  ...crates.filter((c) => licenseTexts(dirname(c.manifest_path)).length === 0).map((c) => `${c.name} ${c.version} (${c.license})`),
  ...npm.filter((p) => licenseTexts(p.dir).length === 0).map((p) => `${p.name} ${p.version} (${p.license})`),
];

const engineLicense = join(root, "src-tauri", "engine", "License.txt");
const sevenZipText = existsSync(engineLicense)
  ? readFileSync(engineLicense, "utf8").trim()
  : "(Esegui `npm run fetch-7zip` per includere il testo ufficiale di License.txt.)";

let md = `# Avvisi di terze parti — PezzaliAPPZipUnzip

PezzaliAPPZipUnzip © 2026 Alessandro Pezzali. Il codice originale dell'app è
distribuito con licenza MIT (vedi \`LICENSE\`). La licenza MIT **non** si
applica ai componenti di terze parti elencati qui, ciascuno con la propria licenza.

File generato da \`scripts/third-party.mjs\`.

## 1. 7-Zip (motore di compressione)

- Autore: Igor Pavlov — https://www.7-zip.org
- Versione inclusa: 26.05, eseguibile ufficiale non modificato (vedi \`engine/VERSION.txt\`)
- Uso: programma separato, avviato come processo esterno. Nessun codice di 7-Zip è
  incorporato o collegato nell'app.
- Licenze: GNU LGPL 2.1 o successiva; GNU LGPL con restrizione unRAR per il
  decompressore RAR; BSD 3-clause (LZFSE, ZSTD); BSD 2-clause (XXH64).
- Codice sorgente corrispondente: https://github.com/ip7z/7zip/releases/download/26.05/7z2605-src.tar.xz
  (SHA-256 0b6080b8c3e7a579161d2d495224c0e51c467576cb4c7048b0ab485ef44cc22e).
  In caso di distribuzione pubblica dell'app, il pacchetto sorgente va reso
  disponibile nello stesso luogo dei binari (es. allegato alla release).
- **Restrizione unRAR:** il codice unRAR non può essere usato per sviluppare un
  archiviatore compatibile con RAR (WinRAR). PezzaliAPPZipUnzip estrae archivi RAR
  ma non li crea.

\`\`\`text
${sevenZipText}
\`\`\`

## 2. Componenti Rust (${crates.length} crate)

| Crate | Versione | Licenza |
|---|---|---|
${crates.map((c) => `| ${c.name} | ${c.version} | ${c.license ?? c.license_file ?? "?"} |`).join("\n")}

I crate con licenza MPL-2.0 sono usati senza modifiche; il loro codice sorgente
è disponibile su https://crates.io.

## 3. Componenti JavaScript dell'interfaccia (${npm.length} pacchetti)

| Pacchetto | Versione | Licenza |
|---|---|---|
${npm.map((p) => `| ${p.name} | ${p.version} | ${p.license} |`).join("\n")}

## 4. Testi delle licenze

`;
for (const [text, users] of groups) {
  md += `### Usato da: ${users.join(", ")}\n\n\`\`\`text\n${text}\n\`\`\`\n\n`;
}
if (missing.length) {
  md += `### Pacchetti senza file di licenza separato\n\nLicenza dichiarata nei metadati del pacchetto (testo standard SPDX applicabile):\n\n${missing.map((m) => `- ${m}`).join("\n")}\n`;
}

writeFileSync(join(root, "THIRD_PARTY_NOTICES.md"), md);
console.log(`✓ THIRD_PARTY_NOTICES.md: ${crates.length} crate, ${npm.length} pacchetti npm, ${groups.size} testi di licenza, ${missing.length} senza file`);
