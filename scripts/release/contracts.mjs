import assert from "node:assert/strict";

export const REPOSITORY = "erivgout/windfall";
export const ALPHA = Object.freeze({
  version: "0.1.0-alpha.1",
  tag: "v0.1.0-alpha.1",
  tagObject: "9e7094f1ec1053cbae83d48067fc594fc593855a",
  commit: "157f96fd068b46adab046a6880ce27ea4b49aea1",
  assets: [
    {
      name: "Windfall_0.1.0-alpha.1_x64-setup.exe",
      size: 9070359,
      sha256:
        "51b1ab3a6d1fa2ad0f6bfb78c9bacf9ecc1bb73a4415dd1c6dfc5db7286a81a3",
    },
    {
      name: "SHA256SUMS.txt",
      size: 103,
      sha256:
        "520617b06f4fb04b1bbd75b51cc23fa23143a7b0df3902b0ecdceb4ad75a78d6",
    },
  ],
});
export const TARGETS = Object.freeze({
  "windows/x64": { triple: "x86_64-pc-windows-msvc", formats: ["nsis"] },
  "macos/aarch64": { triple: "aarch64-apple-darwin", formats: ["dmg"] },
  "linux/x64": { triple: "x86_64-unknown-linux-gnu", formats: ["deb"] },
});
export const BLOCKERS = Object.freeze([
  "build-tool-license-provenance",
  "dependency-corresponding-source-review",
  "dependency-license-review",
  "installer-content-and-native-install-upgrade-review",
  "platform-signing-notarization",
  "updater-security-recovery-and-product-acceptance",
]);
export function check(ok, message) {
  assert.ok(ok, message);
}
export function fields(value, names, label) {
  check(
    value && typeof value === "object" && !Array.isArray(value),
    `${label}: expected object`,
  );
  assert.deepEqual(
    Object.keys(value).sort(),
    [...names].sort(),
    `${label}: missing or unknown fields`,
  );
}
export function string(value, label, max = 512) {
  check(
    typeof value === "string" &&
      value.length > 0 &&
      value.length <= max &&
      !/[\x00-\x1f\x7f]/.test(value),
    `${label}: invalid string`,
  );
  return value;
}
export function commit(value) {
  check(
    /^[a-f0-9]{40}$/.test(value),
    "commit must be a full lowercase 40-character SHA",
  );
  return value;
}
export function digest(value) {
  check(/^[a-f0-9]{64}$/.test(value), "invalid SHA256");
  return value;
}
export function identity(version, channel, sha) {
  string(version, "version", 100);
  commit(sha);
  const m =
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(
      version,
    );
  check(
    m && m.slice(1, 4).every((n) => Number.isSafeInteger(Number(n))),
    "invalid release semantic version (build metadata is not admitted)",
  );
  check(
    !m[4] ||
      m[4]
        .split(".")
        .every((p) => !/^\d+$/.test(p) || p === "0" || !p.startsWith("0")),
    "invalid numeric prerelease identifier",
  );
  check(
    channel === "stable" || channel === "prerelease",
    "unknown release channel",
  );
  check(
    Boolean(m[4]) === (channel === "prerelease"),
    "version/channel mismatch",
  );
  check(
    version !== ALPHA.version && sha !== ALPHA.commit,
    "immutable published alpha cannot be reused",
  );
  const base = m.slice(1, 4).map(Number);
  let newer = null;
  for (let i = 0; i < 3; i++)
    if (base[i] !== [0, 1, 0][i]) {
      newer = base[i] > [0, 1, 0][i];
      break;
    }
  if (newer === null) {
    if (!m[4]) newer = true;
    else {
      const parts = m[4].split(".");
      const alpha = ["alpha", "1"];
      for (let i = 0; i < Math.max(parts.length, alpha.length); i++) {
        if (parts[i] === alpha[i]) continue;
        if (parts[i] === undefined) {
          newer = false;
          break;
        }
        if (alpha[i] === undefined) {
          newer = true;
          break;
        }
        const aNumeric = /^\d+$/.test(parts[i]);
        const bNumeric = /^\d+$/.test(alpha[i]);
        newer =
          aNumeric && bNumeric
            ? BigInt(parts[i]) > BigInt(alpha[i])
            : aNumeric !== bNumeric
              ? !aNumeric
              : parts[i] > alpha[i];
        break;
      }
    }
  }
  check(
    newer === true,
    "candidate version must be newer than the immutable alpha",
  );
  return { repository: REPOSITORY, commit: sha, version, channel };
}
export function mode(value, env = {}) {
  check(
    ["unsigned-development", "signed-release"].includes(value),
    "unknown distribution mode",
  );
  if (value === "signed-release") {
    const missing = [
      "WINDFALL_SIGNING_PROFILE",
      "WINDFALL_SIGNING_VERIFIER",
    ].filter((k) => !env[k]);
    throw new Error(
      missing.length
        ? `signed-release requires operator configuration: ${missing.join(", ")}`
        : "signed-release is gated: approved native signing/notarization verifier integration is not implemented",
    );
  }
}
export function target(platform, architecture) {
  const t = TARGETS[`${platform}/${architecture}`];
  check(t, "unsupported platform/architecture");
  return { platform, architecture, triple: t.triple };
}
export function installerNames(version, platform, architecture) {
  target(platform, architecture);
  if (platform === "windows")
    return [{ format: "nsis", name: `Windfall_${version}_x64-setup.exe` }];
  if (platform === "macos")
    return [{ format: "dmg", name: `Windfall_${version}_aarch64.dmg` }];
  return [{ format: "deb", name: `Windfall_${version}_amd64.deb` }];
}
export function provenance(p) {
  fields(
    p,
    [
      "workflowCommit",
      "workflowPath",
      "runId",
      "runAttempt",
      "runnerImage",
      "tools",
    ],
    "provenance",
  );
  commit(p.workflowCommit);
  check(
    p.workflowPath === ".github/workflows/release-candidate.yml" ||
      p.workflowPath === "local",
    "unknown workflow path",
  );
  for (const k of ["runId", "runAttempt", "runnerImage"]) string(p[k], k);
  fields(
    p.tools,
    ["git", "node", "pnpm", "rustc", "cargo", "tauri"],
    "tool versions",
  );
  for (const v of Object.values(p.tools)) string(v, "tool version");
}
