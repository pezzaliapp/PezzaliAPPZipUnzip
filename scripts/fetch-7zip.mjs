#!/usr/bin/env node
// Scarica il motore 7-Zip ufficiale per il pacchetto dell'app,
// ne verifica l'hash SHA-256 e lo copia in src-tauri/engine/.
//
// Uso: node scripts/fetch-7zip.mjs [--target mac|linux-x64|linux-arm64|win-x64|win-arm64]
//
// Fonte: release ufficiali di Igor Pavlov, https://github.com/ip7z/7zip
// (collegate da https://www.7-zip.org/download.html). Download gratuito.
// Licenza: GNU LGPL 2.1+ con restrizione unRAR, parti BSD 3/2-clause.

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION = "26.05";
const BASE = `https://github.com/ip7z/7zip/releases/download/${VERSION}/`;

// Hash verificati su due fonti: download diretto e digest della release GitHub.
const ASSETS = {
  mac: { file: "7z2605-mac.tar.xz", sha256: "12ef6830317743cfd3f0ab1724bde43fcc270d8b6d8564f8c12c2a2b7f993570", bins: ["7zz"] },
  "linux-x64": { file: "7z2605-linux-x64.tar.xz", sha256: "22cf82fcc6eb7474f1d5bb4f02c0f8ebd573a718089d6833dc30f8350a3b232e", bins: ["7zz"] },
  "linux-arm64": { file: "7z2605-linux-arm64.tar.xz", sha256: "0b71f6cfc34669b3bf69f2347efd3429f77020665003a13b9cdc53b784270421", bins: ["7zz"] },
  "win-x64": { file: "7z2605-x64.exe", sha256: "fd99e86f57d8f429ef5ed045bf98315af962e2309b0f1ba87a147a44c71647d0", bins: ["7z.exe", "7z.dll"] },
  "win-arm64": { file: "7z2605-arm64.exe", sha256: "86d5790d5dd84cab4636c3bb7fab10d3ed7cd3e57e6119401bd2769e28165268", bins: ["7z.exe", "7z.dll"] },
};
const SEVENZR = { file: "7zr.exe", sha256: "9bf582b894dfa665f852d7e9112b02290a1c303c2d95a21fcdc41ffcb03143b7" };
const DOCS = ["License.txt", "readme.txt"];

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const engineDir = join(root, "src-tauri", "engine");

function detectTarget() {
  const i = process.argv.indexOf("--target");
  if (i > 0 && process.argv[i + 1]) return process.argv[i + 1];
  const arch = process.arch === "arm64" ? "arm64" : "x64";
  if (process.platform === "darwin") return "mac";
  if (process.platform === "win32") return `win-${arch}`;
  return `linux-${arch}`;
}

async function download(file, sha256, dir) {
  const dest = join(dir, file);
  console.log(`↓ ${BASE}${file}`);
  const res = await fetch(BASE + file, { redirect: "follow" });
  if (!res.ok) throw new Error(`download fallito: ${res.status} ${res.statusText}`);
  const buf = Buffer.from(await res.arrayBuffer());
  const got = createHash("sha256").update(buf).digest("hex");
  if (got !== sha256) throw new Error(`hash SHA-256 non valido per ${file}\n  atteso: ${sha256}\n  ottenuto: ${got}`);
  writeFileSync(dest, buf);
  console.log(`✓ SHA-256 verificato (${file})`);
  return dest;
}

function run(cmd, args) {
  const r = spawnSync(cmd, args, { stdio: "inherit" });
  if (r.status !== 0) throw new Error(`${cmd} ${args.join(" ")} → codice ${r.status}`);
}

function hostSevenZip() {
  for (const c of ["7zz", "7z", "7za"]) {
    if (spawnSync(c, ["i"], { stdio: "ignore" }).status === 0) return c;
  }
  return null;
}

async function main() {
  const target = detectTarget();
  const asset = ASSETS[target];
  if (!asset) throw new Error(`target sconosciuto: ${target} (validi: ${Object.keys(ASSETS).join(", ")})`);

  const tmp = mkdtempSync(join(tmpdir(), "pezzaliapp-7zip-"));
  try {
    const archive = await download(asset.file, asset.sha256, tmp);
    const out = join(tmp, "out");
    mkdirSync(out);
    if (target.startsWith("win")) {
      // L'installer è un archivio 7z autoestraente: lo si apre senza eseguirlo.
      let tool = process.platform === "win32" ? null : hostSevenZip();
      if (!tool) tool = await download(SEVENZR.file, SEVENZR.sha256, tmp);
      run(tool, ["x", "-y", `-o${out}`, archive, ...asset.bins, ...DOCS]);
    } else {
      run("tar", ["-xf", archive, "-C", out]);
    }

    mkdirSync(engineDir, { recursive: true });
    for (const f of readdirSync(engineDir)) {
      if (f !== "README.md") rmSync(join(engineDir, f), { recursive: true, force: true });
    }
    for (const f of [...asset.bins, ...DOCS]) {
      const src = join(out, f);
      if (!existsSync(src)) throw new Error(`file mancante nel pacchetto 7-Zip: ${f}`);
      copyFileSync(src, join(engineDir, f));
      if (!f.endsWith(".txt") && !f.endsWith(".dll")) chmodSync(join(engineDir, f), 0o755);
    }
    writeFileSync(
      join(engineDir, "VERSION.txt"),
      `7-Zip ${VERSION} (${target})\nFonte: ${BASE}${asset.file}\nSHA-256: ${asset.sha256}\n` +
        `Sorgenti: ${BASE}7z2605-src.tar.xz\nLicenza: GNU LGPL 2.1+ con restrizione unRAR, BSD 3-clause e 2-clause (vedi License.txt)\n`,
    );
    console.log(`✓ motore 7-Zip ${VERSION} per ${target} in ${engineDir}`);
    console.log(readFileSync(join(engineDir, "VERSION.txt"), "utf8"));
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

main().catch((e) => {
  console.error(`✗ ${e.message}`);
  process.exit(1);
});
