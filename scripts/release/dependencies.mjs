import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { check, fields, string } from "./contracts.mjs";
import {
  compare,
  fileRecord,
  inside,
  json,
  parseJson,
  portable,
  rootPath,
  run,
  unique,
} from "./files.mjs";
import { cargoMetadata } from "./source.mjs";

export function cargoLock(text) {
  check(
    /^version = 4\s*$/m.test(text),
    "unsupported/malformed Cargo lock format",
  );
  const blocks = text.split(/^\[\[package\]\]\s*$/m).slice(1);
  check(blocks.length > 0, "Cargo lock has no packages");
  return blocks.map((block) => {
    const values = {};
    for (const key of ["name", "version", "source", "checksum"]) {
      const found = [
        ...block.matchAll(new RegExp(`^${key} = "([^"\\r\\n]+)"\\s*$`, "gm")),
      ];
      check(found.length <= 1, "duplicate Cargo lock property");
      values[key] = found[0]?.[1] ?? null;
    }
    string(values.name, "Cargo package name");
    string(values.version, "Cargo package version");
    if (values.source)
      check(
        values.source.startsWith("registry+") &&
          /^[a-f0-9]{64}$/.test(values.checksum),
        "unsupported Cargo source or missing pinned checksum",
      );
    else
      check(
        values.checksum === null,
        "local Cargo package has contradictory checksum",
      );
    return {
      ecosystem: "cargo",
      name: values.name,
      version: values.version,
      source: values.source ?? "workspace",
      integrity: values.checksum,
    };
  });
}
export function npmLock(text) {
  check(
    /^lockfileVersion: '9\.0'\s*$/m.test(text) &&
      /^importers:\s*$/m.test(text) &&
      /^snapshots:\s*$/m.test(text),
    "unsupported/malformed pnpm lock format",
  );
  const top = [...text.matchAll(/^([A-Za-z][\w-]*):/gm)].map((m) => m[1]);
  check(
    json(top) ===
      json([
        "lockfileVersion",
        "settings",
        "importers",
        "packages",
        "snapshots",
      ]),
    "unsupported pnpm lock sections",
  );
  const section = text
    .split(/^packages:\s*$/m)[1]
    ?.split(/^snapshots:\s*$/m)[0];
  check(section !== undefined, "pnpm packages missing");
  const lines = section.split(/\r?\n/);
  const result = [];
  for (let i = 0; i < lines.length; i++) {
    if (!/^  \S/.test(lines[i])) continue;
    const m = /^  (?:'([^']+)'|([^'\s][^:]*)):\s*$/.exec(lines[i]);
    check(m, "malformed pnpm package key");
    const key = m[1] ?? m[2];
    const at = key.lastIndexOf("@");
    check(at > 0, "malformed pinned npm identity");
    const name = key.slice(0, at);
    const version = key.slice(at + 1);
    check(
      /^(?:@[a-z0-9._-]+\/)?[a-z0-9._-]+$/.test(name) &&
        /^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(version),
      "unsupported npm package identity",
    );
    let body = "";
    for (let j = i + 1; j < lines.length && !/^  \S/.test(lines[j]); j++)
      body += `${lines[j]}\n`;
    const integrity =
      /^    resolution: \{integrity: (sha512-[A-Za-z0-9+/]+={0,2})\}\s*$/m.exec(
        body,
      );
    check(
      integrity && Buffer.from(integrity[1].slice(7), "base64").length === 64,
      "missing/unsupported npm pinned integrity",
    );
    result.push({
      ecosystem: "npm",
      name,
      version,
      source: "npm-registry",
      integrity: integrity[1],
    });
  }
  // An empty packages map is valid for dependency-free fixture repositories.
  return result;
}
const keyOf = (p) => `${p.ecosystem}:${p.name}@${p.version}:${p.source}`;
export function lockedDependencies(root) {
  const result = [
    ...cargoLock(fs.readFileSync(inside(root, "Cargo.lock"), "utf8")),
    ...npmLock(
      fs.readFileSync(inside(root, "apps/desktop/pnpm-lock.yaml"), "utf8"),
    ),
  ].sort((a, b) => compare(keyOf(a), keyOf(b)));
  check(
    new Set(result.map(keyOf)).size === result.length,
    "duplicate locked dependency identity",
  );
  return result;
}
function notices(dir, prefix, output) {
  dir = rootPath(dir);
  const names = fs
    .readdirSync(dir)
    .filter(
      (n) =>
        /^(?:licen[cs]e|notice|copying)(?:[._-].*)?$/i.test(n) &&
        fs.lstatSync(path.join(dir, n)).isFile(),
    );
  const files = [];
  for (const n of names.sort(compare)) {
    portable(n);
    const rec = fileRecord(dir, n);
    const dest = `${prefix}/${n}`;
    const bytes = fs.readFileSync(inside(dir, n));
    output.set(dest, bytes);
    files.push({ ...rec, file: dest });
  }
  return files;
}
function allowedDirectory(dir, roots) {
  const real = fs.realpathSync(dir);
  check(
    roots.some((r) => real === r || real.startsWith(`${r}${path.sep}`)),
    "dependency metadata path outside allowed package stores",
  );
  return rootPath(real);
}
export function dependencyEvidence(root, meta, licenses, allowedRoots) {
  const locks = lockedDependencies(root);
  const copied = new Map();
  check(
    meta?.version === 1 &&
      Array.isArray(meta.packages) &&
      Array.isArray(meta.workspace_members) &&
      meta.resolve &&
      Array.isArray(meta.resolve.nodes),
    "full Cargo dependency graph is missing/malformed",
  );
  check(
    licenses && typeof licenses === "object" && !Array.isArray(licenses),
    "malformed npm license metadata",
  );
  const cargo = new Map();
  for (const p of meta.packages) {
    string(p.name, "Cargo metadata name");
    string(p.version, "Cargo metadata version");
    string(p.id, "Cargo metadata id");
    const key = keyOf({
      ecosystem: "cargo",
      name: p.name,
      version: p.version,
      source: p.source ?? "workspace",
    });
    check(
      !cargo.has(key) && locks.some((l) => keyOf(l) === key),
      "Cargo metadata duplicate or outside pinned lock",
    );
    check(
      p.license === null || typeof p.license === "string",
      "malformed declared Cargo license",
    );
    cargo.set(key, p);
  }
  const ids = new Set(meta.packages.map((p) => p.id));
  for (const node of meta.resolve.nodes) {
    check(
      ids.has(node.id) &&
        Array.isArray(node.dependencies) &&
        node.dependencies.every((id) => ids.has(id)),
      "malformed Cargo dependency graph edges",
    );
  }
  check(
    meta.resolve.nodes.length === ids.size,
    "truncated Cargo dependency graph",
  );
  const npm = new Map();
  for (const group of Object.values(licenses)) {
    check(Array.isArray(group), "malformed npm license group");
    for (const p of group) {
      string(p.name, "npm metadata name");
      check(
        Array.isArray(p.versions) &&
          p.versions.length > 0 &&
          Array.isArray(p.paths) &&
          p.paths.length > 0,
        "truncated npm license metadata",
      );
      check(
        p.license === null || typeof p.license === "string",
        "malformed declared npm license",
      );
      const observed = new Set();
      for (const dir of p.paths) {
        const base = allowedDirectory(dir, allowedRoots.npm);
        const pkg = parseJson(
          fs.readFileSync(inside(base, "package.json"), "utf8"),
        );
        check(
          pkg.name === p.name && p.versions.includes(pkg.version),
          "installed npm identity/metadata mismatch",
        );
        const key = keyOf({
          ecosystem: "npm",
          name: pkg.name,
          version: pkg.version,
          source: "npm-registry",
        });
        check(
          locks.some((l) => keyOf(l) === key),
          "installed npm package outside pinned lock",
        );
        check(!npm.has(key), "duplicate installed npm metadata");
        observed.add(pkg.version);
        // Preserve the installed package's declaration. A summarized pnpm group
        // must never manufacture equivalence or override package metadata.
        npm.set(key, {
          base,
          license: typeof pkg.license === "string" ? pkg.license : null,
        });
      }
      check(
        p.versions.every((v) => observed.has(v)),
        "truncated npm installed paths",
      );
    }
  }
  const packages = locks.map((l, index) => {
    const c = cargo.get(keyOf(l));
    const n = npm.get(keyOf(l));
    let license = c?.license ?? n?.license ?? null;
    if (license !== null) {
      string(license, "declared license");
      if (/^(unknown|unlicensed|undefined)$/i.test(license)) license = null;
    }
    const base = c
      ? allowedDirectory(path.dirname(c.manifest_path), allowedRoots.cargo)
      : n?.base;
    const prefix = `notices/dependencies/${String(index).padStart(5, "0")}`;
    const files = base ? notices(base, prefix, copied) : [];
    if (c && c.source === null && files.length === 0) {
      const rec = fileRecord(root, "LICENSE");
      const dest = `${prefix}/workspace-LICENSE`;
      copied.set(dest, fs.readFileSync(inside(root, "LICENSE")));
      files.push({ ...rec, file: dest });
    }
    if (c?.license_file) {
      const relative = path
        .relative(base, path.resolve(base, c.license_file))
        .split(path.sep)
        .join("/");
      portable(relative);
      const rec = fileRecord(base, relative);
      const dest = `${prefix}/declared-license.txt`;
      copied.set(dest, fs.readFileSync(inside(base, relative)));
      files.push({ ...rec, file: dest });
    }
    return {
      ...l,
      declaredLicense: license,
      provenance: !base
        ? "not-installed-on-this-platform"
        : license && files.length
          ? "declared-unreviewed"
          : "missing-license-or-notice",
      notices: files.sort((a, b) => compare(a.file, b.file)),
    };
  });
  const graph = meta.resolve.nodes
    .map((n) => ({
      package: keyOf({
        ecosystem: "cargo",
        name: meta.packages.find((p) => p.id === n.id).name,
        version: meta.packages.find((p) => p.id === n.id).version,
        source: meta.packages.find((p) => p.id === n.id).source ?? "workspace",
      }),
      dependencies: n.dependencies
        .map((id) => {
          const p = meta.packages.find((p) => p.id === id);
          return keyOf({
            ecosystem: "cargo",
            name: p.name,
            version: p.version,
            source: p.source ?? "workspace",
          });
        })
        .sort(compare),
    }))
    .sort((a, b) => compare(a.package, b.package));
  const inventory = {
    schemaVersion: 1,
    format: "windfall-lock-backed-sbom",
    scope: "all-lock-entries-including-build-and-platform-optional",
    licenseReview: "pending",
    packages,
    cargoGraph: graph,
  };
  validateDependencies(root, inventory);
  return { inventory, notices: copied };
}
export function captureDependencies(root) {
  const meta = cargoMetadata(root, true);
  const licenses = parseJson(
    run(
      "pnpm",
      ["licenses", "list", "--json"],
      path.join(root, "apps/desktop"),
    ),
  );
  const cargoHome = process.env.CARGO_HOME ?? path.join(os.homedir(), ".cargo");
  const registry = path.join(cargoHome, "registry/src");
  const npmRoot = path.join(root, "apps/desktop/node_modules");
  return dependencyEvidence(root, meta, licenses, {
    cargo: [
      rootPath(root),
      ...(fs.existsSync(registry) ? [rootPath(registry)] : []),
    ],
    npm: [rootPath(npmRoot)],
  });
}
export function validateDependencies(root, inventory) {
  fields(
    inventory,
    [
      "schemaVersion",
      "format",
      "scope",
      "licenseReview",
      "packages",
      "cargoGraph",
    ],
    "dependency inventory",
  );
  check(
    inventory.schemaVersion === 1 &&
      inventory.format === "windfall-lock-backed-sbom" &&
      inventory.scope ===
        "all-lock-entries-including-build-and-platform-optional" &&
      inventory.licenseReview === "pending",
    "unknown dependency inventory state",
  );
  const locks = lockedDependencies(root);
  check(
    Array.isArray(inventory.packages) &&
      inventory.packages.length === locks.length,
    "truncated dependency inventory",
  );
  const names = [];
  for (let i = 0; i < locks.length; i++) {
    const p = inventory.packages[i];
    fields(
      p,
      [
        "ecosystem",
        "name",
        "version",
        "source",
        "integrity",
        "declaredLicense",
        "provenance",
        "notices",
      ],
      "dependency",
    );
    for (const k of Object.keys(locks[i]))
      check(
        p[k] === locks[i][k],
        "dependency identity/integrity differs from lock",
      );
    check(
      p.declaredLicense === null ||
        (typeof p.declaredLicense === "string" && p.declaredLicense.length > 0),
      "invalid dependency license",
    );
    check(
      [
        "declared-unreviewed",
        "missing-license-or-notice",
        "not-installed-on-this-platform",
      ].includes(p.provenance) && Array.isArray(p.notices),
      "unknown dependency provenance",
    );
    check(
      p.provenance !== "declared-unreviewed" ||
        (p.declaredLicense && p.notices.length),
      "contradictory dependency provenance",
    );
    check(
      p.provenance !== "missing-license-or-notice" ||
        !p.declaredLicense ||
        p.notices.length === 0,
      "contradictory missing license/notice claim",
    );
    check(
      p.provenance !== "not-installed-on-this-platform" ||
        (p.declaredLicense === null && p.notices.length === 0),
      "contradictory missing dependency provenance",
    );
    for (const n of p.notices) {
      fields(n, ["file", "size", "sha256"], "dependency notice");
      check(
        n.file.startsWith(
          `notices/dependencies/${String(i).padStart(5, "0")}/`,
        ),
        "dependency notice ownership mismatch",
      );
      names.push(n.file);
    }
  }
  unique(names);
  const keys = new Set(locks.filter((l) => l.ecosystem === "cargo").map(keyOf));
  check(
    Array.isArray(inventory.cargoGraph) && inventory.cargoGraph.length > 0,
    "missing dependency graph",
  );
  const graphKeys = new Set();
  for (const node of inventory.cargoGraph) {
    fields(node, ["package", "dependencies"], "dependency graph");
    check(
      keys.has(node.package) &&
        !graphKeys.has(node.package) &&
        Array.isArray(node.dependencies) &&
        node.dependencies.every((k) => keys.has(k)),
      "malformed dependency graph",
    );
    graphKeys.add(node.package);
  }
  const installed = inventory.packages
    .filter(
      (p) =>
        p.ecosystem === "cargo" &&
        p.provenance !== "not-installed-on-this-platform",
    )
    .map(keyOf)
    .sort(compare);
  check(
    json([...graphKeys].sort(compare)) === json(installed),
    "truncated Cargo dependency graph inventory",
  );
}
