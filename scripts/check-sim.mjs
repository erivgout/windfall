#!/usr/bin/env node
// Checks that the checked-in WebAssembly document matches the Rust it is built from.
//
//   node scripts/check-sim.mjs           fail if the module is stale (for CI; needs no Rust)
//   node scripts/check-sim.mjs --write   record the current sources (scripts/build-sim.sh does this)
//
// No dependencies. The module is apps/desktop/src/lib/ipc/sim/windfall_sim.wasm. Beside it,
// windfall_sim.wasm.json records a hash of the sources it was built from and a hash of the
// module itself. The sources are the src folder and Cargo.toml of windfall-sim and of every
// workspace crate it depends on.

import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const CRATES = join(ROOT, "crates");
const WASM = join(ROOT, "apps/desktop/src/lib/ipc/sim/windfall_sim.wasm");
const RECORD = `${WASM}.json`;
const REBUILD = "Run scripts/build-sim.sh and commit windfall_sim.wasm and windfall_sim.wasm.json.";

function sha256(data) {
  return createHash("sha256").update(data).digest("hex");
}

/** The workspace crates a crate's Cargo.toml lists under [dependencies]. */
function workspaceDependencies(crate) {
  const manifest = readFileSync(join(CRATES, crate, "Cargo.toml"), "utf8");
  const names = [];
  let section = "";
  for (const line of manifest.split(/\r?\n/)) {
    const header = line.match(/^\s*\[(.+)\]\s*$/);
    if (header) section = header[1];
    const dependency = line.match(/^\s*(windfall-[a-z0-9-]+)\b/);
    if (section === "dependencies" && dependency) names.push(dependency[1]);
  }
  return names;
}

/** windfall-sim and everything in the workspace it is built from, sorted. */
function inputCrates() {
  const found = new Set();
  const pending = ["windfall-sim"];
  for (let crate = pending.pop(); crate !== undefined; crate = pending.pop()) {
    if (found.has(crate)) continue;
    found.add(crate);
    pending.push(...workspaceDependencies(crate));
  }
  return [...found].sort();
}

/** Every file below a folder, as sorted paths with forward slashes. */
function filesBelow(folder, prefix) {
  return readdirSync(folder, { withFileTypes: true })
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .flatMap((entry) => {
      const path = `${prefix}/${entry.name}`;
      return entry.isDirectory() ? filesBelow(join(folder, entry.name), path) : [path];
    });
}

function hashInputs() {
  const hash = createHash("sha256");
  for (const crate of inputCrates()) {
    const files = [`${crate}/Cargo.toml`, ...filesBelow(join(CRATES, crate, "src"), `${crate}/src`)];
    for (const file of files) {
      // Git checks text out with CRLF on some machines. The hash must not care.
      const text = readFileSync(join(CRATES, file), "utf8").replace(/\r\n/g, "\n");
      hash.update(`${file}\0${text.length}\0${text}`);
    }
  }
  return hash.digest("hex");
}

function fail(message) {
  console.error(`check-sim: ${message}`);
  console.error(`check-sim: ${REBUILD}`);
  process.exit(1);
}

if (!existsSync(WASM)) fail("windfall_sim.wasm is missing.");
const wasm = readFileSync(WASM);
const current = { inputs: hashInputs(), wasm: sha256(wasm), bytes: wasm.length };

if (process.argv.includes("--write")) {
  writeFileSync(RECORD, `${JSON.stringify(current, null, 2)}\n`);
  console.log(`check-sim: recorded ${current.bytes} bytes built from ${inputCrates().join(", ")}`);
  process.exit(0);
}

if (!existsSync(RECORD)) fail("windfall_sim.wasm.json is missing.");
const recorded = JSON.parse(readFileSync(RECORD, "utf8"));
if (recorded.wasm !== current.wasm) {
  fail("windfall_sim.wasm is not the module windfall_sim.wasm.json describes.");
}
if (recorded.inputs !== current.inputs) {
  fail(
    `windfall_sim.wasm is stale: the Rust it is built from (${inputCrates().join(", ")}) has changed since it was built.`,
  );
}
console.log(`check-sim: windfall_sim.wasm is current (${current.bytes} bytes)`);
