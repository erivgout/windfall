import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import {
  ALPHA,
  REPOSITORY,
  check,
  fields,
  identity,
  mode,
} from "./contracts.mjs";
import {
  compare,
  fileRecord,
  inside,
  json,
  parseJson,
  portable,
  rootPath,
  run,
  sha256,
  unique,
  walk,
} from "./files.mjs";

export function cargoMetadata(root, full = false) {
  return parseJson(
    run(
      "cargo",
      [
        "metadata",
        "--locked",
        "--format-version",
        "1",
        ...(full ? [] : ["--no-deps"]),
      ],
      root,
    ),
  );
}
export function sourceEvidence(root, requested) {
  root = rootPath(root);
  fields(
    requested,
    ["commit", "version", "channel", "mode"],
    "requested source",
  );
  const id = identity(requested.version, requested.channel, requested.commit);
  mode(requested.mode);
  check(
    run("git", ["rev-parse", "HEAD"], root) === id.commit,
    "checked-out source commit mismatch",
  );
  try {
    run("git", ["diff", "--no-ext-diff", "--quiet", "HEAD", "--"], root);
    run(
      "git",
      ["diff", "--cached", "--no-ext-diff", "--quiet", "HEAD", "--"],
      root,
    );
  } catch {
    throw new Error("source must be clean, including tracked/index changes");
  }
  check(
    run("git", ["ls-files", "--others", "--exclude-standard", "-z"], root) ===
      "",
    "source must be clean, including untracked nonignored files",
  );
  const endings = new Map(
    run("git", ["ls-files", "--eol", "-z"], root)
      .split("\0")
      .filter(Boolean)
      .map((line) => {
        const at = line.indexOf("\t");
        return [line.slice(at + 1), line.slice(0, at)];
      }),
  );
  const gitBlob = (bytes) =>
    createHash("sha1")
      .update(`blob ${bytes.length}\0`)
      .update(bytes)
      .digest("hex");
  const tracked = run("git", ["ls-tree", "-r", "-z", "HEAD"], root)
    .split("\0")
    .filter(Boolean)
    .map((line) => {
      const m = /^(100644|100755) blob ([a-f0-9]{40})\t(.+)$/.exec(line);
      check(
        m,
        "source symlinks, gitlinks and unsupported entries are forbidden",
      );
      const name = portable(m[3]);
      const bytes = fs.readFileSync(inside(root, name));
      const eol = endings.get(name) ?? "";
      const normalized =
        /i\/lf\s+w\/crlf\s/.test(eol) && !eol.includes("attr/-text")
          ? bytes.filter((byte, i) => !(byte === 13 && bytes[i + 1] === 10))
          : bytes;
      check(
        gitBlob(bytes) === m[2] || gitBlob(normalized) === m[2],
        "working source bytes differ from immutable commit (including assume-unchanged files)",
      );
      return {
        file: name,
        size: bytes.length,
        sha256: sha256(bytes),
        gitBlob: m[2],
        mode: m[1],
      };
    })
    .sort((a, b) => compare(a.file, b.file));
  unique(tracked.map((f) => f.file));
  const nullDevice = process.platform === "win32" ? "NUL" : "/dev/null";
  const attrs = run(
    "git",
    [
      "-c",
      `core.attributesFile=${nullDevice}`,
      "check-attr",
      "--cached",
      "-z",
      "export-ignore",
      "export-subst",
      "--stdin",
    ],
    root,
    { input: tracked.map((f) => `${f.file}\0`).join("") },
  ).split("\0");
  check(
    attrs.pop() === "" && attrs.length === tracked.length * 6,
    "incomplete archive attribute evidence",
  );
  for (let i = 0; i < attrs.length; i += 3)
    check(
      ["unspecified", "unset"].includes(attrs[i + 2]),
      "source archive export-ignore/export-subst is not admitted",
    );
  const pkg = parseJson(
    fs.readFileSync(inside(root, "apps/desktop/package.json"), "utf8"),
  );
  const conf = parseJson(
    fs.readFileSync(
      inside(root, "apps/desktop/src-tauri/tauri.conf.json"),
      "utf8",
    ),
  );
  const toml = fs.readFileSync(inside(root, "Cargo.toml"), "utf8");
  const section = toml.split("[workspace.package]")[1]?.split(/\n\s*\[/)[0];
  const versions = [
    ...(section ?? "").matchAll(/^version\s*=\s*"([^"]+)"\s*$/gm),
  ];
  check(
    versions.length === 1 &&
      versions[0][1] === id.version &&
      pkg.version === id.version &&
      conf.version === id.version,
    "Cargo workspace/package/Tauri versions disagree with requested version",
  );
  check(
    pkg.name === "windfall-desktop" &&
      conf.productName === "Windfall" &&
      conf.identifier === "org.windfall.Windfall",
    "unexpected product identity",
  );
  const meta = cargoMetadata(root);
  check(
    meta.version === 1 &&
      Array.isArray(meta.packages) &&
      Array.isArray(meta.workspace_members) &&
      meta.workspace_members.length > 0,
    "malformed Cargo workspace metadata",
  );
  for (const member of meta.workspace_members) {
    const p = meta.packages.find((p) => p.id === member);
    check(p && p.version === id.version, "Cargo member version mismatch");
  }
  const desktop = meta.packages.find((p) => p.name === "windfall-desktop");
  check(desktop, "desktop Cargo package is missing");
  const binaries = desktop.targets.filter((t) => t.kind.includes("bin"));
  check(
    binaries.length === 1 && binaries[0].name === "windfall-desktop",
    "unaccepted desktop binary/helper packaging contract",
  );
  const tree = run("git", ["rev-parse", "HEAD^{tree}"], root);
  return {
    schemaVersion: 1,
    identity: id,
    mode: requested.mode,
    tree,
    files: tracked,
    snapshotSha256: sha256(json(tracked)),
  };
}
export function checkCapture(root, requested, captured) {
  fields(
    captured,
    ["schemaVersion", "identity", "mode", "tree", "files", "snapshotSha256"],
    "source capture",
  );
  const now = sourceEvidence(root, requested);
  check(
    json(now) === json(captured),
    "source changed after capture or capture metadata mismatch",
  );
  return now;
}
export function sourceArchive(root, sha) {
  // Git's tar records the exact commit; do not archive the mutable working tree.
  return run(
    "git",
    [
      "-c",
      "tar.umask=0002",
      "-c",
      `core.attributesFile=${process.platform === "win32" ? "NUL" : "/dev/null"}`,
      "archive",
      "--format=tar",
      `--prefix=Windfall-${sha}/`,
      sha,
    ],
    root,
    { binary: true },
  );
}
export function resources(root) {
  const conf = parseJson(
    fs.readFileSync(
      inside(root, "apps/desktop/src-tauri/tauri.conf.json"),
      "utf8",
    ),
  );
  check(
    conf.bundle?.active === true && !conf.bundle.externalBin?.length,
    "unaccepted packaged helper contract",
  );
  check(
    json(conf.bundle.resources) ===
      json({ "../../../content/factory/": "factory/" }),
    "unaccepted resource mapping (helper/model/resource seam requires parent integration)",
  );
  for (const platform of ["windows", "linux", "macos"]) {
    check(
      !fs.existsSync(
        path.join(root, `apps/desktop/src-tauri/tauri.${platform}.conf.json`),
      ),
      "platform-specific config requires an accepted packaging contract",
    );
  }
  for (const platform of ["macOS", "linux"]) {
    const p = conf.bundle[platform];
    check(
      !p?.files || Object.keys(p.files).length === 0,
      "unaccepted platform resource files",
    );
    if (platform === "linux")
      for (const b of ["deb", "rpm", "appimage"])
        check(
          !p?.[b]?.files || Object.keys(p[b].files).length === 0,
          "unaccepted Linux resource files",
        );
  }
  const table = fs.readFileSync(
    inside(root, "crates/windfall-factory/src/lib.rs"),
    "utf8",
  );
  const sounds = [
    ...table.matchAll(/sound\(Category::\w+, "[^"]+", "([^"]+\.wav)"/g),
  ].map((m) => m[1]);
  const count = /static SOUNDS: \[Sound; (\d+)\]/.exec(table);
  check(
    count && sounds.length === Number(count[1]) && sounds.length > 0,
    "factory manifest cannot be read completely",
  );
  unique(sounds);
  const names = walk(root, "content/factory");
  const expected = [...sounds, "LICENSE.md", "README.md"]
    .map((n) => `content/factory/${n}`)
    .sort(compare);
  check(
    json(names) === json(expected),
    "factory resource inventory differs from source manifest",
  );
  const license = fs.readFileSync(
    inside(root, "content/factory/LICENSE.md"),
    "utf8",
  );
  check(
    license.includes("CC0 1.0 Universal") &&
      license.includes("GPL-3.0-or-later"),
    "factory provenance dedication is missing",
  );
  return {
    schemaVersion: 1,
    mapping: { source: "content/factory", destination: "factory" },
    license: "CC0-1.0",
    generationSource: "crates/windfall-factory/src/lib.rs",
    helpers: "not-admitted",
    models: "not-admitted",
    packagedContents: "native-inspection-required",
    files: names.map((n) => ({
      ...fileRecord(root, n),
      destination: `factory/${n.slice("content/factory/".length)}`,
    })),
  };
}
export function sourceNotices(root, captured) {
  const names = captured.files
    .map((f) => f.file)
    .filter((n) =>
      /(?:^|\/)(?:LICENSE(?:[-._][^/]*)?|NOTICE(?:[-._][^/]*)?|COPYING(?:[-._][^/]*)?)$/i.test(
        n,
      ),
    );
  check(
    names.includes("LICENSE") && names.includes("content/factory/LICENSE.md"),
    "source/factory notices are missing",
  );
  return names
    .sort(compare)
    .map((n) => ({ ...fileRecord(root, n), output: `notices/source/${n}` }));
}
export async function githubIdentity(requested, token = process.env.GH_TOKEN) {
  identity(requested.version, requested.channel, requested.commit);
  check(
    token,
    "read-only GitHub token is required for remote identity preflight",
  );
  async function api(route) {
    const r = await fetch(
      `https://api.github.com/repos/${REPOSITORY}/${route}`,
      {
        headers: {
          Accept: "application/vnd.github+json",
          Authorization: `Bearer ${token}`,
          "X-GitHub-Api-Version": "2022-11-28",
        },
        signal: AbortSignal.timeout(30000),
      },
    );
    check(
      r.ok,
      `GitHub identity lookup failed (${r.status}); refusing release identity`,
    );
    return r.json();
  }
  const tag = await api(`git/ref/tags/${ALPHA.tag}`);
  check(
    tag.object?.type === "tag" && tag.object.sha === ALPHA.tagObject,
    "immutable alpha tag changed",
  );
  const object = await api(`git/tags/${ALPHA.tagObject}`);
  check(
    object.object?.type === "commit" && object.object.sha === ALPHA.commit,
    "immutable alpha dereference changed",
  );
  const alphaRelease = await api(`releases/tags/${ALPHA.tag}`);
  check(
    alphaRelease.tag_name === ALPHA.tag &&
      !alphaRelease.draft &&
      alphaRelease.prerelease,
    "immutable alpha release identity changed",
  );
  for (const expected of ALPHA.assets) {
    const asset = alphaRelease.assets?.find((a) => a.name === expected.name);
    check(
      asset &&
        asset.size === expected.size &&
        (!asset.digest || asset.digest === `sha256:${expected.sha256}`),
      "immutable alpha asset metadata changed",
    );
  }
  const names = [requested.version, `v${requested.version}`].map((n) =>
    n.toLowerCase(),
  );
  for (const ref of await api(
    `git/matching-refs/tags/${encodeURIComponent(requested.version)}`,
  ))
    check(
      !names.includes(ref.ref.slice("refs/tags/".length).toLowerCase()),
      "release tag identity is already reserved",
    );
  for (const ref of await api(
    `git/matching-refs/tags/${encodeURIComponent(`v${requested.version}`)}`,
  ))
    check(
      !names.includes(ref.ref.slice("refs/tags/".length).toLowerCase()),
      "release tag identity is already reserved",
    );
  for (let page = 1; ; page++) {
    check(
      page <= 100,
      "release pagination limit reached; refusing incomplete lookup",
    );
    const releases = await api(`releases?per_page=100&page=${page}`);
    check(Array.isArray(releases), "malformed release lookup");
    for (const release of releases)
      check(
        !names.includes(String(release.tag_name).toLowerCase()),
        "published/draft release identity already exists",
      );
    if (releases.length < 100) break;
  }
  return {
    repository: REPOSITORY,
    checkedVersion: requested.version,
    alphaTagObject: ALPHA.tagObject,
    state: "unpublished-identity-observed",
  };
}
