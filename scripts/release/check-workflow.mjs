import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { check, fields, TARGETS } from "./contracts.mjs";
import { parseJson } from "./files.mjs";

// JSON is a YAML 1.2 subset. Keeping this workflow in that subset permits strict
// dependency-free parsing, including duplicate-field detection.
const workflow = fileURLToPath(
  new URL("../../.github/workflows/release-candidate.yml", import.meta.url),
);
export function validateWorkflow(file = workflow) {
  const text = fs.readFileSync(file, "utf8");
  const w = parseJson(text);
  check(
    (text.match(/"uses"\s*:/g) ?? []).length ===
      Object.values(w.jobs)
        .flatMap((j) => j.steps)
        .filter((s) => s.uses).length,
    "truncated/duplicate workflow step fields",
  );
  fields(
    w,
    ["name", "on", "permissions", "concurrency", "env", "defaults", "jobs"],
    "workflow",
  );
  fields(w.on, ["workflow_dispatch"], "workflow triggers");
  check(
    JSON.stringify(w.permissions) === JSON.stringify({ contents: "read" }),
    "workflow must have read-only repository permissions",
  );
  fields(
    w.on.workflow_dispatch.inputs,
    ["source_commit", "version", "channel", "mode"],
    "dispatch inputs",
  );
  check(
    w.concurrency["cancel-in-progress"] === false &&
      w.jobs.candidate.needs === "preflight",
    "candidate must wait for preflight without cancelling a peer",
  );
  const pins = new Set([
    "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
    "actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1",
    "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
    "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9",
  ]);
  for (const job of Object.values(w.jobs)) {
    check(!job.permissions, "jobs cannot override read-only permissions");
    for (const step of job.steps) {
      check(!step.permissions, "step cannot alter permissions");
      if (step.uses)
        check(pins.has(step.uses), "unverified workflow action pin");
      if (step.uses?.startsWith("actions/checkout@"))
        check(
          step.with["persist-credentials"] === false,
          "checkout must not persist credentials",
        );
      if (step.run)
        check(
          !step.run.includes("${{") &&
            !/gh (release|workflow|api)|curl|\|\s*(sh|bash)|git push/.test(
              step.run,
            ),
          "unsafe input interpolation or publication/remote script command",
        );
    }
  }
  const rows = w.jobs.candidate.strategy.matrix.include;
  check(
    rows.length === Object.keys(TARGETS).length &&
      new Set(rows.map((r) => `${r.platform}/${r.architecture}`)).size ===
        rows.length,
    "candidate target matrix is incomplete/duplicated",
  );
  for (const row of rows)
    check(
      TARGETS[`${row.platform}/${row.architecture}`]?.triple === row.target &&
        TARGETS[`${row.platform}/${row.architecture}`].formats.join(",") ===
          row.bundles,
      "bundle target matrix differs from filename contract",
    );
  const steps = w.jobs.candidate.steps;
  const native = steps.find((s) => s.name.startsWith("Build native"));
  check(
    native.run.includes("--no-sign") && native.run.includes("-- --locked"),
    "native build must explicitly disable signing and lock Cargo",
  );
  check(
    steps.some((s) => s.run?.includes("pnpm install --frozen-lockfile")),
    "frozen desktop install is required",
  );
  const verification = steps.findIndex(
    (s) => s.name === "Independent verification before upload",
  );
  const uploads = steps
    .map((s, i) => [s, i])
    .filter(([s]) => s.uses?.startsWith("actions/upload-artifact@"));
  check(
    uploads.length === 1 &&
      verification >= 0 &&
      uploads[0][1] > verification &&
      uploads[0][0].with.overwrite === false &&
      uploads[0][0].with["if-no-files-found"] === "error" &&
      uploads[0][0].with["include-hidden-files"] === true,
    "only complete verified no-clobber workflow candidate uploads are allowed",
  );
  return w;
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    validateWorkflow();
    console.log("Candidate workflow contract passed (JSON/YAML 1.2 subset).");
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
