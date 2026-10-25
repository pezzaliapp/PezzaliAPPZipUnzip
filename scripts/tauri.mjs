#!/usr/bin/env node
// Avvia la CLI di Tauri inclusa nel progetto. Per `tauri build`:
// - verifica che il motore 7-Zip sia in src-tauri/engine (npm run fetch-7zip);
// - sostituisce nei binari i percorsi locali (cartella utente, progetto)
//   con prefissi neutri, così il pacchetto non contiene il nome dell'utente.

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");
const args = process.argv.slice(2);
const env = { ...process.env };

if (args[0] === "build") {
  const engine = join(root, "src-tauri", "engine");
  if (!["7zz", "7z.exe"].some((f) => existsSync(join(engine, f)))) {
    console.error("✗ motore 7-Zip assente in src-tauri/engine: eseguire prima `npm run fetch-7zip`.");
    process.exit(1);
  }
  // Il prefisso più lungo va per ultimo: rustc applica l'ultima regola che corrisponde.
  const remaps = [`${homedir()}=~`, `${root}=.`];
  if (env.CARGO_HOME) remaps.push(`${env.CARGO_HOME}=/cargo`);
  // Forma codificata (separatore 0x1f): funziona anche con spazi nei percorsi.
  const base = env.CARGO_ENCODED_RUSTFLAGS
    ? env.CARGO_ENCODED_RUSTFLAGS.split("\x1f")
    : (env.RUSTFLAGS ?? "").split(" ").filter(Boolean);
  env.CARGO_ENCODED_RUSTFLAGS = [...base, ...remaps.map((r) => `--remap-path-prefix=${r}`)].join("\x1f");
  delete env.RUSTFLAGS;
}

const r = spawnSync(process.execPath, [cli, ...args], { stdio: "inherit", env });
process.exit(r.status ?? 1);
