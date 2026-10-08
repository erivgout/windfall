#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { check, commit, identity, mode, target } from "./contracts.mjs";
import { createCandidate, verifyCandidate } from "./candidate.mjs";
import { atomicNew, json, load, rootPath, run } from "./files.mjs";
import { githubIdentity, resources, sourceEvidence } from "./source.mjs";

function args(argv) {
  const values = {};
  const command = argv.shift();
  const allowed = [
    "source",
    "commit",
    "version",
    "channel",
    "mode",
    "output",
    "capture",
    "bundle-root",
    "candidate",
    "platform",
    "architecture",
    "github",
  ];
  while (argv.length) {
    const name = argv.shift();
    check(
      name.startsWith("--") &&
        allowed.includes(name.slice(2)) &&
        !(name.slice(2) in values),
      "unknown/duplicate CLI option",
    );
    const key = name.slice(2);
    if (key === "github") values[key] = true;
    else {
      check(argv.length && !argv[0].startsWith("--"), `missing --${key} value`);
      values[key] = argv.shift();
    }
  }
  check(
    ["preflight", "capture", "candidate", "verify"].includes(command),
    "usage: cli.mjs preflight|capture|candidate|verify --source DIR --commit SHA --version VERSION --channel prerelease|stable --mode unsigned-development [command options]",
  );
  const specific = {
    preflight: ["github"],
    capture: ["output"],
    candidate: [
      "output",
      "capture",
      "bundle-root",
      "platform",
      "architecture",
      "github",
    ],
    verify: ["candidate"],
  };
  for (const k of Object.keys(values))
    check(
      [
        "source",
        "commit",
        "version",
        "channel",
        "mode",
        ...specific[command],
      ].includes(k),
      `--${k} is not valid for ${command}`,
    );
  for (const k of ["source", "commit", "version", "channel", "mode"])
    check(values[k], `missing --${k}`);
  for (const k of specific[command].filter((k) => k !== "github"))
    check(values[k], `missing --${k}`);
  return { command, values };
}
function buildProvenance(source) {
  const toolsRoot = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../..",
  );
  const workflowCommit =
    process.env.GITHUB_WORKFLOW_SHA ??
    run("git", ["rev-parse", "HEAD"], toolsRoot);
  commit(workflowCommit);
  const tools = {};
  for (const executable of ["git", "pnpm", "rustc", "cargo"])
    tools[executable] = run(executable, ["--version"], source);
  tools.node = process.version;
  tools.tauri = run(
    "pnpm",
    ["exec", "tauri", "--version"],
    path.join(source, "apps/desktop"),
  );
  check(
    tools.node === "v26.4.0" &&
      tools.pnpm === "11.1.2" &&
      /^rustc 1\.99\.0\b/.test(tools.rustc) &&
      /^cargo 1\.99\.0\b/.test(tools.cargo) &&
      /^tauri(?:-cli)? 2\.12\.1$/.test(tools.tauri),
    "observed build tools differ from the accepted pinned toolchain",
  );
  return {
    workflowCommit,
    workflowPath:
      process.env.GITHUB_ACTIONS === "true"
        ? ".github/workflows/release-candidate.yml"
        : "local",
    runId: process.env.GITHUB_RUN_ID ?? "local",
    runAttempt: process.env.GITHUB_RUN_ATTEMPT ?? "1",
    runnerImage: process.env.ImageVersion
      ? `${process.env.ImageOS ?? process.platform}/${process.env.ImageVersion}`
      : `${process.platform}/${process.arch}`,
    tools,
  };
}
export async function main(argv = process.argv.slice(2)) {
  const { command, values: v } = args([...argv]);
  const requested = {
    commit: v.commit,
    version: v.version,
    channel: v.channel,
    mode: v.mode,
  };
  identity(v.version, v.channel, v.commit);
  mode(v.mode, process.env);
  const source = rootPath(v.source);
  if (command === "verify") {
    verifyCandidate({ source, requested, candidate: v.candidate });
    return;
  }
  const captured = sourceEvidence(source, requested);
  resources(source);
  if (v.github) await githubIdentity(requested);
  if (command === "preflight") return;
  if (command === "capture") {
    atomicNew(v.output, json(captured));
    return;
  }
  target(v.platform, v.architecture);
  const native = { win32: "windows", darwin: "macos", linux: "linux" }[
    process.platform
  ];
  check(
    v.platform === native &&
      v.architecture === (process.arch === "arm64" ? "aarch64" : process.arch),
    "candidate collection must run on the declared native OS/architecture",
  );
  const build = buildProvenance(source);
  const capture = load(v.capture);
  createCandidate({
    source,
    requested,
    capture,
    bundleRoot: v["bundle-root"],
    output: v.output,
    build,
    platform: v.platform,
    architecture: v.architecture,
  });
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main()
    .then(() => console.log("Release candidate gate passed."))
    .catch((error) => {
      // Errors contain contract descriptions only; never dump metadata/env/subprocess logs.
      console.error(`Release candidate gate failed: ${error.message}`);
      process.exitCode = 1;
    });
}
