import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { BLOCKERS } from "./contracts.mjs";
import { load, sha256 } from "./files.mjs";
import {
  checkActionProvenance,
  inspectBuffers,
  tarFiles,
  verifyBlob,
} from "./action-provenance.mjs";

const cli = fileURLToPath(new URL("./action-provenance.mjs", import.meta.url));
const evidenceFile = fileURLToPath(
  new URL("./action-provenance.json", import.meta.url),
);
const hash = (algo, bytes, encoding = "hex") =>
  createHash(algo).update(bytes).digest(encoding);
const code = Buffer.from(
  "// Authored byte fixture; not upstream code or a license grant.\n",
);
function archive(entries) {
  const blocks = [];
  for (const [name, bytes, type = "0"] of entries) {
    const header = Buffer.alloc(512);
    header.write(name);
    header.write(
      `${bytes.length.toString(8).padStart(11, "0")}\0`,
      124,
      "ascii",
    );
    header.fill(32, 148, 156);
    header.write(type, 156);
    const checksum = [...header].reduce((n, b) => n + b, 0);
    header.write(`${checksum.toString(8).padStart(6, "0")}\0 `, 148, "ascii");
    blocks.push(
      header,
      bytes,
      Buffer.alloc((512 - (bytes.length % 512)) % 512),
    );
  }
  blocks.push(Buffer.alloc(1024));
  return Buffer.concat(blocks);
}
function fixture(entries) {
  const tarball = gzipSync(
    archive(
      entries ?? [
        ["package/index.js", code],
        [
          "package/package.json",
          Buffer.from('{"name":"buffers","version":"0.1.1"}'),
        ],
        [
          "package/README.markdown",
          Buffer.from("Authored fixture with no grant."),
        ],
      ],
    ),
  );
  const integrity = `sha512-${hash("sha512", tarball, "base64")}`;
  const url = "https://registry.npmjs.org/buffers/-/buffers-0.1.1.tgz";
  return {
    tarball,
    lockEntry: { version: "0.1.1", resolved: url, integrity },
    metadata: {
      name: "buffers",
      version: "0.1.1",
      dist: { tarball: url, integrity, shasum: hash("sha1", tarball) },
    },
    distributions: [
      {
        path: "dist/upload/index.js",
        bytes: Buffer.concat([
          Buffer.from("prefix\n"),
          code,
          Buffer.from("suffix\n"),
        ]),
      },
    ],
    reviewed: "reviewed:\n  npm:\n  - buffers # MIT\n",
  };
}

test("exact package bytes are attributed without treating a maintainer comment as a grant", () => {
  const f = fixture();
  const first = inspectBuffers(f);
  assert.deepEqual(inspectBuffers(f), first);
  assert.equal(first.registryLicense, null);
  assert.equal(first.packageLicense, null);
  assert.equal(first.upstreamReviewComment, "MIT");
  assert.equal(first.grantVerification, "not-established");
  assert.deepEqual(first.licenseNoticeFiles, []);
  assert.deepEqual(first.distributionSourceMatches, [
    {
      path: "dist/upload/index.js",
      source: "package/index.js",
      byteOffset: 7,
      size: code.length,
      sha256: sha256(code),
    },
  ]);
});

test("package/hash/source identity tampering fails before attribution", () => {
  const f = fixture();
  for (const changed of [
    { ...f, tarball: Buffer.concat([f.tarball, Buffer.from("tamper")]) },
    { ...f, lockEntry: { ...f.lockEntry, integrity: "sha512-invalid" } },
    { ...f, metadata: { ...f.metadata, version: "0.1.2" } },
    {
      ...f,
      lockEntry: {
        ...f.lockEntry,
        resolved: "https://example.invalid/package.tgz",
      },
    },
    {
      ...f,
      metadata: {
        ...f.metadata,
        dist: { ...f.metadata.dist, shasum: "0".repeat(40) },
      },
    },
  ])
    assert.throws(() => inspectBuffers(changed));
});

test("missing, altered and ambiguous distribution bytes cannot establish source attribution", () => {
  const f = fixture();
  for (const bytes of [
    Buffer.from("unrelated"),
    Buffer.from(code.toString().toUpperCase()),
    Buffer.concat([code, code]),
  ]) {
    assert.throws(
      () =>
        inspectBuffers({
          ...f,
          distributions: [{ path: "dist/upload/index.js", bytes }],
        }),
      /attribution/,
    );
  }
});

test("tar inspection rejects malformed headers, traversal, links, collisions and truncated bytes", () => {
  for (const entries of [
    [["package/../escape", code]],
    [["/outside", code]],
    [["package/link", code, "2"]],
    [
      ["package/index.js", code],
      ["package/index.js", code],
    ],
    [
      ["package/Case", code],
      ["package/case", code],
    ],
  ])
    assert.throws(() => tarFiles(gzipSync(archive(entries))));
  const tar = archive([["package/index.js", code]]);
  const changed = Buffer.from(tar);
  changed[124] = 57;
  assert.throws(() => tarFiles(gzipSync(changed)));
  assert.throws(() => tarFiles(gzipSync(tar.subarray(0, 600))));
  assert.throws(() => tarFiles(gzipSync(tar.subarray(0, tar.length - 512))));
});

test("available notices/declarations stay separate from unresolved copyright-holder grant", () => {
  const f = fixture([
    ["package/index.js", code],
    [
      "package/package.json",
      Buffer.from('{"name":"buffers","version":"0.1.1","license":"MIT"}'),
    ],
    [
      "package/LICENSE",
      Buffer.from("Authored notice, not a recovered upstream grant."),
    ],
  ]);
  const result = inspectBuffers(f);
  assert.equal(result.packageLicense, "MIT");
  assert.deepEqual(result.licenseNoticeFiles, ["package/LICENSE"]);
  assert.equal(result.grantVerification, "not-established");
});

test("Git blob attribution binds exact fetched bytes and size", () => {
  const bytes = Buffer.from("authored source bytes\n");
  const entry = {
    path: "src/fixture.ts",
    size: bytes.length,
    sha: hash(
      "sha1",
      Buffer.concat([Buffer.from(`blob ${bytes.length}\0`), bytes]),
    ),
  };
  assert.equal(verifyBlob(bytes, entry).sha256, sha256(bytes));
  assert.throws(() =>
    verifyBlob(Buffer.concat([bytes, Buffer.from("x")]), entry),
  );
  assert.throws(() => verifyBlob(bytes, { ...entry, sha: "0".repeat(40) }));
});

test("checked evidence cannot erase CI closure, claim publication or truncate the dependency inventory", () => {
  const evidence = load(evidenceFile);
  checkActionProvenance(evidence);
  assert.ok(BLOCKERS.includes("build-tool-license-provenance"));
  for (const mutate of [
    (e) => {
      e.applicability.publicationEligible = true;
    },
    (e) => {
      e.applicability.toolchainLicenseClosure = "verified";
    },
    (e) => {
      e.buffers.grantVerification = "MIT";
    },
    (e) => {
      e.actions[0].commit = "0".repeat(40);
    },
    (e) => {
      e.actions[3].runtimeDependencies.pop();
    },
    (e) => {
      e.actions[3].files = e.actions[3].files.filter(
        (f) => f.path !== "LICENSE",
      );
    },
    (e) => {
      e.buffers.distributionSourceMatches[0].sha256 = "0".repeat(64);
    },
    (e) => {
      e.actions[0].runtimeDependencies[0].resolved =
        "https://example.invalid/not-pinned";
    },
  ]) {
    const changed = structuredClone(evidence);
    mutate(changed);
    assert.throws(() => checkActionProvenance(changed));
  }
});

test("read-only evidence CLI succeeds offline and invalid commands have nonzero status", () => {
  const success = spawnSync(process.execPath, [cli, "check"], {
    encoding: "utf8",
    windowsHide: true,
  });
  assert.equal(success.status, 0, success.stderr);
  assert.match(success.stdout, /closure remains unresolved/);
  const invalid = spawnSync(process.execPath, [cli, "publish"], {
    encoding: "utf8",
    windowsHide: true,
  });
  assert.equal(invalid.status, 1);
  assert.match(invalid.stderr, /usage:/);
});
