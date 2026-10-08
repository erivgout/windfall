import fs from "node:fs";
import path from "node:path";
import { createHash, randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import { check, string } from "./contracts.mjs";

export const sha256 = (bytes) =>
  createHash("sha256").update(bytes).digest("hex");
export const json = (value) => `${JSON.stringify(value, null, 2)}\n`;
export const compare = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
export function portable(name) {
  string(name, "relative path", 1000);
  check(
    !name.includes("\\") &&
      !path.posix.isAbsolute(name) &&
      !/^[A-Za-z]:/.test(name),
    "absolute/backslash path is forbidden",
  );
  for (const p of name.split("/")) {
    check(
      p &&
        p !== "." &&
        p !== ".." &&
        !/[<>:"|?*]/.test(p) &&
        !/[. ]$/.test(p) &&
        !/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(p),
      "unsafe or nonportable path",
    );
  }
  return name;
}
export function unique(names) {
  const seen = new Set();
  for (const n of names) {
    portable(n);
    const key = n.toLowerCase();
    check(!seen.has(key), "duplicate/case-colliding path");
    seen.add(key);
  }
}
export function rootPath(dir) {
  const abs = path.resolve(dir);
  const real = fs.realpathSync(abs);
  check(
    real.toLowerCase() === abs.toLowerCase(),
    "root resolves through a symlink",
  );
  let cursor = abs;
  while (true) {
    check(!fs.lstatSync(cursor).isSymbolicLink(), "symlink root/ancestor");
    const next = path.dirname(cursor);
    if (next === cursor) break;
    cursor = next;
  }
  check(fs.statSync(abs).isDirectory(), "root is not a directory");
  return abs;
}
export function inside(root, name) {
  portable(name);
  root = rootPath(root);
  const abs = path.resolve(root, ...name.split("/"));
  check(abs.startsWith(`${root}${path.sep}`), "path escaped root");
  let cursor = root;
  for (const part of name.split("/")) {
    cursor = path.join(cursor, part);
    check(!fs.lstatSync(cursor).isSymbolicLink(), "symlink file/ancestor");
  }
  check(
    fs.realpathSync(abs).toLowerCase() === abs.toLowerCase(),
    "file resolves outside root",
  );
  return abs;
}
export function fileRecord(root, name) {
  const abs = inside(root, name);
  const stat = fs.lstatSync(abs);
  check(
    stat.isFile() && stat.size > 0 && Number.isSafeInteger(stat.size),
    "artifact must be a nonempty regular file",
  );
  const bytes = fs.readFileSync(abs);
  const after = fs.lstatSync(abs);
  check(
    after.size === stat.size &&
      after.mtimeMs === stat.mtimeMs &&
      bytes.length === stat.size,
    "file changed during capture",
  );
  return { file: name, size: bytes.length, sha256: sha256(bytes) };
}
export function walk(root, prefix = "") {
  const dir = prefix ? inside(root, prefix) : rootPath(root);
  const result = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const n = prefix ? `${prefix}/${e.name}` : e.name;
    portable(n);
    check(!e.isSymbolicLink(), "symlink inventory entry");
    if (e.isDirectory()) result.push(...walk(root, n));
    else {
      check(e.isFile(), "non-regular inventory entry");
      result.push(n);
    }
  }
  unique(result);
  return result.sort(compare);
}
export function run(command, args, cwd, { binary = false, input } = {}) {
  // No shell interpolation, including on Windows. Invoke pnpm's JS entrypoint there.
  if (command === "pnpm" && process.platform === "win32") {
    const found = spawnSync("where.exe", ["pnpm.cmd"], { encoding: "utf8" });
    check(found.status === 0, "pnpm.cmd not found");
    const wrapper = found.stdout.trim().split(/\r?\n/)[0];
    const entry = path.join(
      path.dirname(wrapper),
      "node_modules/pnpm/bin/pnpm.cjs",
    );
    check(fs.existsSync(entry), "pnpm JS entrypoint not found");
    args = [entry, ...args];
    command = process.execPath;
  }
  const r = spawnSync(command, args, {
    cwd,
    encoding: binary ? undefined : "utf8",
    input,
    maxBuffer: 256 * 1024 * 1024,
    windowsHide: true,
  });
  check(
    !r.error && r.status === 0,
    `${path.basename(command)} failed (status ${r.status ?? "unavailable"}); no environment or command output disclosed`,
  );
  return binary ? r.stdout : r.stdout.trim();
}
export function parseJson(text) {
  const value = JSON.parse(text);
  const tokens =
    text.match(
      /"(?:\\.|[^"\\])*"|[{}\[\]:,]|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null/g,
    ) ?? [];
  let position = 0;
  function scan() {
    const first = tokens[position++];
    if (first === "{") {
      const seen = new Set();
      if (tokens[position] === "}") {
        position++;
        return;
      }
      while (true) {
        const key = JSON.parse(tokens[position++]);
        check(!seen.has(key), "duplicate JSON field");
        seen.add(key);
        check(tokens[position++] === ":", "malformed JSON object");
        scan();
        const end = tokens[position++];
        if (end === "}") break;
        check(end === ",", "malformed JSON object separator");
      }
    } else if (first === "[") {
      if (tokens[position] === "]") {
        position++;
        return;
      }
      while (true) {
        scan();
        const end = tokens[position++];
        if (end === "]") break;
        check(end === ",", "malformed JSON array separator");
      }
    }
  }
  scan();
  check(position === tokens.length, "truncated JSON token inventory");
  return value;
}
export function load(file) {
  return parseJson(fs.readFileSync(file, "utf8"));
}
export function writeNew(root, name, bytes) {
  portable(name);
  const abs = path.join(root, ...name.split("/"));
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  rootPath(path.dirname(abs));
  fs.writeFileSync(abs, bytes, { flag: "wx" });
}
export function atomicNew(file, bytes) {
  const parent = rootPath(path.dirname(path.resolve(file)));
  portable(path.basename(file));
  const temp = path.join(parent, `.release-${randomUUID()}.tmp`);
  fs.writeFileSync(temp, bytes, { flag: "wx", mode: 0o600 });
  try {
    fs.linkSync(temp, file);
  } finally {
    fs.unlinkSync(temp);
  }
}
export function stage(output, build) {
  const abs = path.resolve(output);
  const parent = rootPath(path.dirname(abs));
  portable(path.basename(abs));
  check(
    !fs.existsSync(abs),
    "candidate output already exists; refusing to clobber",
  );
  const staging = fs.mkdtempSync(path.join(parent, ".release-stage-"));
  const owner = randomUUID();
  fs.writeFileSync(path.join(staging, ".owner"), owner, { flag: "wx" });
  try {
    build(staging);
    // mkdir exclusively reserves the destination. Each link is atomic/no-clobber;
    // manifest is the completion marker and is linked last. Never rename over a competitor.
    fs.mkdirSync(abs);
    const names = walk(staging)
      .filter((n) => n !== ".owner")
      .sort(
        (a, b) =>
          (a === "manifest.json") - (b === "manifest.json") || compare(a, b),
      );
    for (const n of names) {
      const dest = path.join(abs, ...n.split("/"));
      fs.mkdirSync(path.dirname(dest), { recursive: true });
      rootPath(path.dirname(dest));
      fs.linkSync(inside(staging, n), dest);
    }
    return abs;
  } finally {
    check(
      path.dirname(path.resolve(staging)) === parent &&
        path.basename(staging).startsWith(".release-stage-") &&
        fs.readFileSync(path.join(staging, ".owner"), "utf8") === owner,
      "cleanup ownership mismatch",
    );
    rootPath(staging);
    walk(staging);
    fs.rmSync(staging, { recursive: true });
  }
}
