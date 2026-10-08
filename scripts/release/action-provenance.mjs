import assert from "node:assert/strict";
import path from "node:path";
import { createHash } from "node:crypto";
import { gunzipSync } from "node:zlib";
import { fileURLToPath } from "node:url";
import { check, fields } from "./contracts.mjs";
import {
  atomicNew,
  compare,
  json,
  load,
  parseJson,
  portable,
  sha256,
  unique,
} from "./files.mjs";
import { validateWorkflow } from "./check-workflow.mjs";

// This audit reads data only. It never installs or evaluates action/package code.
export const ACTIONS = Object.freeze(
  [
    ["checkout", "3d3c42e5aac5ba805825da76410c181273ba90b1"],
    ["setup-node", "949feb2413d6458794dcd2491c4babbbce0c15c1"],
    ["cache", "55cc8345863c7cc4c66a329aec7e433d2d1c52a9"],
    ["upload-artifact", "cf430e030ddbb5b0abf93d22962f4752f3646cd9"],
  ].map(([repository, commit]) =>
    Object.freeze({ repository: `actions/${repository}`, commit }),
  ),
);
const evidenceFile = fileURLToPath(
  new URL("./action-provenance.json", import.meta.url),
);
const hash = (algorithm, bytes, encoding = "hex") =>
  createHash(algorithm).update(bytes).digest(encoding);
const blob = (bytes) =>
  hash("sha1", Buffer.concat([Buffer.from(`blob ${bytes.length}\0`), bytes]));
const rawUrl = (action, file) =>
  `https://raw.githubusercontent.com/${action.repository}/${action.commit}/${file}`;
const BUFFER_URL = "https://registry.npmjs.org/buffers/-/buffers-0.1.1.tgz";
const BUFFER_METADATA = "https://registry.npmjs.org/buffers/0.1.1";
const BUFFER_REPOSITORY = "https://api.github.com/repos/substack/node-buffers";
const applicability = Object.freeze({
  scope: "external-ci-toolchain",
  productDependency: false,
  actionCodeConveyedByCandidateCollector: false,
  nativeContentsReview: "pending",
  toolchainLicenseClosure: "not-established",
  publicationEligible: false,
});

export function verifyBlob(bytes, entry) {
  check(
    Buffer.isBuffer(bytes) &&
      bytes.length === entry.size &&
      blob(bytes) === entry.sha,
    "pinned Git blob size/hash mismatch",
  );
  return {
    path: entry.path,
    size: bytes.length,
    gitBlob: entry.sha,
    sha256: sha256(bytes),
  };
}

// Deliberately narrow reader for this six-file historical tarball, not an extractor.
// Reject extensions, links, duplicate paths, truncation and invalid header checksums.
export function tarFiles(compressed) {
  const tar = gunzipSync(compressed, { maxOutputLength: 2 * 1024 * 1024 });
  const result = new Map();
  let offset = 0;
  let terminated = false;
  function octal(bytes) {
    const s = bytes.toString("ascii").replace(/\0.*$/, "").trim();
    check(/^[0-7]+$/.test(s), "invalid tar octal field");
    return parseInt(s, 8);
  }
  while (offset + 512 <= tar.length) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every((b) => b === 0)) {
      check(
        tar.length >= offset + 1024 &&
          tar.subarray(offset).every((b) => b === 0),
        "invalid tar terminator/trailing data",
      );
      terminated = true;
      break;
    }
    const name = header.subarray(0, 100).toString("utf8").split("\0")[0];
    portable(name);
    check(
      name.startsWith("package/") && !result.has(name),
      "duplicate/outside-package tar path",
    );
    check(
      header[156] === 0 || header[156] === 48,
      "tar link/extension/non-file is forbidden",
    );
    check(
      header.subarray(345, 500).every((b) => b === 0),
      "tar extended prefix is unsupported",
    );
    const checksum = [...header].reduce(
      (n, b, i) => n + (i >= 148 && i < 156 ? 32 : b),
      0,
    );
    check(
      checksum === octal(header.subarray(148, 156)),
      "tar header checksum mismatch",
    );
    const size = octal(header.subarray(124, 136));
    const next = offset + 512 + Math.ceil(size / 512) * 512;
    check(size > 0 && next <= tar.length, "truncated/empty tar file");
    result.set(
      name,
      Buffer.from(tar.subarray(offset + 512, offset + 512 + size)),
    );
    offset = next;
  }
  check(terminated && result.size > 0, "truncated tar inventory");
  unique([...result.keys()]);
  return result;
}

export function inspectBuffers({
  lockEntry,
  metadata,
  tarball,
  distributions,
  reviewed,
}) {
  check(
    metadata.name === "buffers" &&
      metadata.version === "0.1.1" &&
      lockEntry.version === "0.1.1",
    "buffers identity mismatch",
  );
  check(
    metadata.dist.tarball === BUFFER_URL && lockEntry.resolved === BUFFER_URL,
    "buffers source URL mismatch",
  );
  const integrity = `sha512-${hash("sha512", tarball, "base64")}`;
  check(
    integrity === lockEntry.integrity &&
      integrity === metadata.dist.integrity &&
      hash("sha1", tarball) === metadata.dist.shasum,
    "buffers tarball integrity mismatch",
  );
  const files = tarFiles(tarball);
  const pkg = parseJson(
    files.get("package/package.json")?.toString() ?? "null",
  );
  check(
    pkg?.name === "buffers" && pkg.version === "0.1.1",
    "buffers package declaration mismatch",
  );
  const code = files.get("package/index.js");
  check(code, "buffers source missing");
  const matches = distributions.map(({ path: name, bytes }) => {
    const offset = bytes.indexOf(code);
    check(
      offset >= 0 && bytes.indexOf(code, offset + 1) === -1,
      "buffers exact source attribution missing/ambiguous",
    );
    return {
      path: name,
      source: "package/index.js",
      byteOffset: offset,
      size: code.length,
      sha256: sha256(code),
    };
  });
  const notices = [...files.keys()]
    .filter((p) => /(?:^|\/)(?:licen[cs]e|copying|notice)(?:[._-]|$)/i.test(p))
    .sort(compare);
  const reviewClaim = /^\s*- buffers # (.+)$/m.exec(reviewed)?.[1] ?? null;
  return {
    name: "buffers",
    version: "0.1.1",
    metadataUrl: BUFFER_METADATA,
    tarballUrl: BUFFER_URL,
    tarballSha256: sha256(tarball),
    integrity,
    declaredRepository: metadata.repository?.url ?? null,
    registryLicense: metadata.license ?? metadata.licenses ?? null,
    packageLicense: pkg.license ?? pkg.licenses ?? null,
    upstreamReviewComment: reviewClaim,
    licenseNoticeFiles: notices,
    // A review comment is a declaration by the action maintainer, not a recovered
    // copyright-holder notice or a grant for this exact historical package.
    grantVerification: "not-established",
    files: [...files]
      .sort(([a], [b]) => compare(a, b))
      .map(([path, bytes]) => ({
        path,
        size: bytes.length,
        sha256: sha256(bytes),
      })),
    distributionSourceMatches: matches,
  };
}

function runtimeLock(lock) {
  check(
    lock.lockfileVersion === 3 &&
      lock.packages?.[""] &&
      typeof lock.packages === "object",
    "unsupported/truncated action lock",
  );
  return Object.entries(lock.packages)
    .filter(([name, p]) => name && !p.dev)
    .sort(([a], [b]) => compare(a, b))
    .map(([path, p]) => {
      portable(path);
      check(
        path.includes("node_modules/") &&
          typeof p.version === "string" &&
          typeof p.integrity === "string",
        "unresolved action runtime dependency",
      );
      const url = new URL(p.resolved);
      check(
        url.origin === "https://registry.npmjs.org" &&
          !url.username &&
          !url.password &&
          !url.search &&
          !url.hash,
        "non-registry action dependency",
      );
      const name = decodeURIComponent(url.pathname.split("/-/")[0].slice(1));
      check(
        /^(@[a-z0-9._-]+\/)?[a-z0-9._-]+$/.test(name),
        "invalid registry dependency identity",
      );
      check(
        p.name === undefined || p.name === name,
        "npm alias/source identity mismatch",
      );
      check(
        /^(sha512-[A-Za-z0-9+/]{86}==|sha1-[A-Za-z0-9+/]{27}=)$/.test(
          p.integrity,
        ),
        "invalid action dependency integrity",
      );
      return {
        path,
        name,
        version: p.version,
        resolved: p.resolved,
        integrity: p.integrity,
        license: p.license ?? null,
      };
    });
}

// Fixed public upstream URLs only; no tokens/env credentials and no redirects.
// Bound reads prevent unexpected HTML, archive bombs or unbounded source input.
async function publicRead(url, limit, allow404 = false) {
  const response = await fetch(url, {
    redirect: "error",
    signal: AbortSignal.timeout(20000),
    headers: { "User-Agent": "windfall-release-provenance" },
  });
  if (allow404 && response.status === 404) return { status: 404, bytes: null };
  check(
    response.status === 200,
    `public provenance read failed (HTTP ${response.status})`,
  );
  const parts = [];
  let size = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    check(size <= limit, "public provenance input exceeds limit");
    parts.push(chunk);
  }
  return { status: 200, bytes: Buffer.concat(parts) };
}
async function batches(items, fn) {
  const result = [];
  for (let i = 0; i < items.length; i += 6)
    result.push(...(await Promise.all(items.slice(i, i + 6).map(fn))));
  return result;
}

export async function collectActionProvenance() {
  const w = validateWorkflow();
  const uses = [
    ...new Set(
      Object.values(w.jobs)
        .flatMap((j) => j.steps)
        .filter((s) => s.uses)
        .map((s) => s.uses),
    ),
  ].sort(compare);
  assert.deepEqual(
    uses,
    ACTIONS.map((a) => `${a.repository}@${a.commit}`).sort(compare),
    "workflow pins differ from audited action set",
  );
  const reports = [];
  let upload;
  for (const action of ACTIONS) {
    const treeUrl = `https://api.github.com/repos/${action.repository}/git/trees/${action.commit}?recursive=1`;
    const tree = parseJson(
      (await publicRead(treeUrl, 2 * 1024 * 1024)).bytes.toString(),
    );
    check(
      tree.truncated === false && Array.isArray(tree.tree),
      "truncated/malformed pinned repository tree",
    );
    const entries = tree.tree
      .filter(
        (e) =>
          e.type === "blob" &&
          ([
            "LICENSE",
            "action.yml",
            "package.json",
            "package-lock.json",
            ".licensed.yml",
          ].includes(e.path) ||
            e.path.startsWith(".licenses/") ||
            e.path.startsWith("src/") ||
            /^dist\/(?:[^/]+\/)*(?:index\.js|package\.json)$/.test(e.path)),
      )
      .sort((a, b) => compare(a.path, b.path));
    unique(entries.map((e) => e.path));
    const data = new Map();
    const files = await batches(entries, async (entry) => {
      check(
        /^[a-f0-9]{40}$/.test(entry.sha) &&
          Number.isSafeInteger(entry.size) &&
          entry.size > 0,
        "invalid pinned tree blob",
      );
      const url = rawUrl(action, entry.path);
      const bytes = (await publicRead(url, 8 * 1024 * 1024)).bytes;
      data.set(entry.path, bytes);
      return { ...verifyBlob(bytes, entry), url };
    });
    for (const name of [
      "LICENSE",
      "action.yml",
      "package.json",
      "package-lock.json",
      ".licensed.yml",
    ])
      check(data.has(name), "missing action license/source metadata");
    const text = (name) => data.get(name).toString();
    const pkg = parseJson(text("package.json"));
    check(
      pkg.license === "MIT" &&
        text("LICENSE").includes("The MIT License (MIT)") &&
        text("LICENSE").includes("copyright notice and this permission notice"),
      "action license grant changed",
    );
    const definition = text("action.yml");
    check(
      /^ +using: (?:'node24'|"node24"|node24)\s*$/m.test(definition),
      "action runtime changed",
    );
    const entrypoints = [
      ...definition.matchAll(
        /^ +(main|pre|post): (?:'([^']+)'|"([^"]+)"|(\S+))\s*$/gm,
      ),
    ].map((m) => ({ phase: m[1], path: portable(m[2] ?? m[3] ?? m[4]) }));
    check(
      entrypoints.some((e) => e.phase === "main") &&
        entrypoints.every((e) => data.has(e.path)),
      "missing action executable distribution",
    );
    const dependencies = runtimeLock(parseJson(text("package-lock.json")));
    reports.push({
      ...action,
      treeUrl,
      version: pkg.version,
      repositoryLicense: "MIT",
      runtime: "node24",
      entrypoints,
      files,
      runtimeDependencies: dependencies,
      noticeReview: "pending",
      correspondingSourceReview: "pending",
    });
    if (action.repository === "actions/upload-artifact")
      upload = { data, dependencies };
  }
  const lockEntry = upload.dependencies.find((p) => p.name === "buffers");
  check(lockEntry, "buffers audit target absent from upload action");
  const metadata = parseJson(
    (await publicRead(BUFFER_METADATA, 1024 * 1024)).bytes.toString(),
  );
  const tarball = (await publicRead(BUFFER_URL, 1024 * 1024)).bytes;
  const buffers = inspectBuffers({
    lockEntry,
    metadata,
    tarball,
    reviewed: upload.data.get(".licensed.yml").toString(),
    distributions: ["dist/upload/index.js", "dist/merge/index.js"].map(
      (path) => ({ path, bytes: upload.data.get(path) }),
    ),
  });
  buffers.repositoryCheck = {
    url: BUFFER_REPOSITORY,
    httpStatus: (await publicRead(BUFFER_REPOSITORY, 1024 * 1024, true)).status,
  };
  const uniquePackages = new Map();
  for (const action of reports)
    for (const p of action.runtimeDependencies) {
      const key = `${p.name}@${p.version}`;
      const prior = uniquePackages.get(key);
      check(
        !prior ||
          (prior.integrity === p.integrity &&
            prior.resolved === p.resolved &&
            prior.license === p.license),
        "contradictory action dependency provenance",
      );
      uniquePackages.set(key, p);
    }
  return {
    schemaVersion: 1,
    kind: "pinned-action-source-provenance",
    applicability,
    uniqueRuntimePackages: uniquePackages.size,
    missingLockLicenseDeclarations: [...uniquePackages.values()]
      .filter((p) => p.license === null)
      .map((p) => `${p.name}@${p.version}`)
      .sort(compare),
    actions: reports,
    buffers,
  };
}

export function checkActionProvenance(evidence = load(evidenceFile)) {
  const workflow = validateWorkflow();
  assert.deepEqual(
    [
      ...new Set(
        Object.values(workflow.jobs)
          .flatMap((j) => j.steps)
          .filter((s) => s.uses)
          .map((s) => s.uses),
      ),
    ].sort(compare),
    ACTIONS.map((a) => `${a.repository}@${a.commit}`).sort(compare),
    "workflow pins differ from audited action set",
  );
  fields(
    evidence,
    [
      "schemaVersion",
      "kind",
      "applicability",
      "uniqueRuntimePackages",
      "missingLockLicenseDeclarations",
      "actions",
      "buffers",
    ],
    "action evidence",
  );
  check(
    evidence.schemaVersion === 1 &&
      evidence.kind === "pinned-action-source-provenance",
    "unknown action evidence schema",
  );
  assert.deepEqual(
    evidence.applicability,
    applicability,
    "CI provenance cannot assert product/license/publication approval",
  );
  assert.deepEqual(
    evidence.actions.map(({ repository, commit }) => ({ repository, commit })),
    ACTIONS,
    "action evidence pin mismatch",
  );
  for (const a of evidence.actions) {
    fields(
      a,
      [
        "repository",
        "commit",
        "treeUrl",
        "version",
        "repositoryLicense",
        "runtime",
        "entrypoints",
        "files",
        "runtimeDependencies",
        "noticeReview",
        "correspondingSourceReview",
      ],
      "action inventory",
    );
    check(
      a.repositoryLicense === "MIT" &&
        a.runtime === "node24" &&
        a.noticeReview === "pending" &&
        a.correspondingSourceReview === "pending",
      "action closure cannot be inferred from root license",
    );
    check(
      a.treeUrl ===
        `https://api.github.com/repos/${a.repository}/git/trees/${a.commit}?recursive=1`,
      "unpinned action source tree URL",
    );
    unique(a.files.map((f) => f.path));
    for (const name of [
      "LICENSE",
      "action.yml",
      "package.json",
      "package-lock.json",
      ".licensed.yml",
    ])
      check(
        a.files.some((f) => f.path === name),
        "truncated action license/source metadata",
      );
    for (const f of a.files) {
      fields(f, ["path", "size", "gitBlob", "sha256", "url"], "action file");
      check(
        f.url === rawUrl(a, f.path) &&
          /^[a-f0-9]{40}$/.test(f.gitBlob) &&
          /^[a-f0-9]{64}$/.test(f.sha256) &&
          Number.isSafeInteger(f.size) &&
          f.size > 0,
        "invalid action file evidence",
      );
    }
    check(
      a.entrypoints.length > 0 &&
        a.entrypoints.every((e) => a.files.some((f) => f.path === e.path)),
      "truncated action entrypoint inventory",
    );
    for (const e of a.entrypoints) {
      fields(e, ["phase", "path"], "action entrypoint");
      check(
        ["main", "pre", "post"].includes(e.phase),
        "unknown action execution phase",
      );
    }
    check(
      a.runtimeDependencies.length > 0,
      "truncated action dependency inventory",
    );
    unique(a.runtimeDependencies.map((p) => p.path));
    for (const p of a.runtimeDependencies)
      fields(
        p,
        ["path", "name", "version", "resolved", "integrity", "license"],
        "action dependency",
      );
    assert.deepEqual(
      a.runtimeDependencies,
      runtimeLock({
        lockfileVersion: 3,
        packages: Object.fromEntries([
          ["", {}],
          ...a.runtimeDependencies.map((p) => [p.path, p]),
        ]),
      }),
      "malformed/unsorted action runtime inventory",
    );
  }
  const uniquePackages = new Set(
    evidence.actions.flatMap((a) =>
      a.runtimeDependencies.map((p) => `${p.name}@${p.version}`),
    ),
  );
  check(
    evidence.uniqueRuntimePackages === uniquePackages.size &&
      uniquePackages.size === 174,
    "truncated action runtime inventory",
  );
  const missing = [
    ...new Set(
      evidence.actions.flatMap((a) =>
        a.runtimeDependencies
          .filter((p) => p.license === null)
          .map((p) => `${p.name}@${p.version}`),
      ),
    ),
  ].sort(compare);
  assert.deepEqual(
    evidence.missingLockLicenseDeclarations,
    missing,
    "missing license declaration inventory mismatch",
  );
  assert.deepEqual(
    missing,
    ["buffers@0.1.1"],
    "unexpected missing action license declaration",
  );
  fields(
    evidence.buffers,
    [
      "name",
      "version",
      "metadataUrl",
      "tarballUrl",
      "tarballSha256",
      "integrity",
      "declaredRepository",
      "registryLicense",
      "packageLicense",
      "upstreamReviewComment",
      "licenseNoticeFiles",
      "grantVerification",
      "files",
      "distributionSourceMatches",
      "repositoryCheck",
    ],
    "buffers evidence",
  );
  check(
    evidence.buffers.name === "buffers" &&
      evidence.buffers.version === "0.1.1" &&
      evidence.buffers.metadataUrl === BUFFER_METADATA &&
      evidence.buffers.tarballUrl === BUFFER_URL &&
      /^[a-f0-9]{64}$/.test(evidence.buffers.tarballSha256),
    "buffers evidence identity mismatch",
  );
  check(
    evidence.buffers.registryLicense === null &&
      evidence.buffers.packageLicense === null &&
      evidence.buffers.grantVerification === "not-established" &&
      evidence.buffers.upstreamReviewComment === "MIT" &&
      evidence.buffers.licenseNoticeFiles.length === 0,
    "buffers declaration/text distinction changed; new source review required",
  );
  check(
    evidence.buffers.distributionSourceMatches.length === 2 &&
      evidence.buffers.files.length === 6,
    "truncated buffers source attribution",
  );
  unique(evidence.buffers.files.map((f) => f.path));
  const source = evidence.buffers.files.find(
    (f) => f.path === "package/index.js",
  );
  const action = evidence.actions.find(
    (a) => a.repository === "actions/upload-artifact",
  );
  assert.deepEqual(
    evidence.buffers.distributionSourceMatches.map((m) => m.path),
    ["dist/upload/index.js", "dist/merge/index.js"],
    "buffers distribution attribution mismatch",
  );
  for (const m of evidence.buffers.distributionSourceMatches) {
    fields(
      m,
      ["path", "source", "byteOffset", "size", "sha256"],
      "buffers distribution match",
    );
    const dist = action.files.find((f) => f.path === m.path);
    check(
      source &&
        dist &&
        m.source === source.path &&
        m.size === source.size &&
        m.sha256 === source.sha256 &&
        Number.isSafeInteger(m.byteOffset) &&
        m.byteOffset >= 0 &&
        m.byteOffset + m.size <= dist.size,
      "contradictory buffers attribution",
    );
  }
  return evidence;
}

export async function main(args) {
  if (args.length === 1 && args[0] === "check") {
    checkActionProvenance();
    console.log(
      "Pinned CI action evidence shape/policy passed; license closure remains unresolved.",
    );
  } else if (args.length === 1 && args[0] === "reproduce") {
    const actual = checkActionProvenance(await collectActionProvenance());
    assert.deepEqual(
      actual,
      checkActionProvenance(),
      "official pinned source evidence changed",
    );
    console.log(
      "Official pinned repository/package bytes reproduce the checked action evidence; buffers grant remains unresolved.",
    );
  } else if (
    args.length === 3 &&
    args[0] === "capture" &&
    args[1] === "--output"
  ) {
    // The exclusive output helper refuses an existing file; no silent inventory replacement.
    atomicNew(
      args[2],
      json(checkActionProvenance(await collectActionProvenance())),
    );
    console.log(
      "Pinned action evidence captured without executing remote code; review remains pending.",
    );
  } else
    throw new Error(
      "usage: action-provenance.mjs check | reproduce | capture --output NEW_FILE",
    );
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    await main(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
