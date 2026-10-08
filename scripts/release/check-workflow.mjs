import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { check, fields, TARGETS } from "./contracts.mjs";
import { parseJson } from "./files.mjs";

// JSON/YAML syntax does not establish GitHub expression-context validity.
// These bounded scope checks follow GitHub's context-availability table and the
// official workflow-parser 0.3.61 schema. They are not a full server validator.
const GLOBAL = ["github", "inputs", "vars"];
const JOB = [...GLOBAL, "needs", "strategy", "matrix"];
const STEP = [...JOB, "job", "runner", "env", "steps"];
const FUNCTIONS = [
  "contains",
  "startswith",
  "endswith",
  "format",
  "join",
  "tojson",
  "fromjson",
];

function expressions(
  value,
  contexts,
  location,
  special = [],
  implicit = false,
) {
  if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value))
      expressions(child, contexts, `${location}.${key}`, special);
    return;
  }
  if (typeof value !== "string") return;
  if (implicit && !value.includes("${{")) value = `\u0024{{ ${value} }}`;
  for (let offset = 0; ; ) {
    const start = value.indexOf("${{", offset);
    if (start < 0) return;
    let quoted = false;
    let body = "";
    let end = start + 3;
    for (; end < value.length; end++) {
      const character = value[end];
      if (character === "'") {
        if (quoted && value[end + 1] === "'") {
          end++;
          continue;
        }
        quoted = !quoted;
        body += " ";
      } else if (!quoted && value.slice(end, end + 2) === "}}") break;
      else body += quoted ? " " : character;
    }
    check(
      end < value.length && !quoted,
      `unterminated expression at ${location}`,
    );
    check(
      body.trim().length > 0 || value.slice(start + 3, end).includes("'"),
      `empty expression at ${location}`,
    );
    check(!body.includes('"'), `unsupported expression quoting at ${location}`);
    body = body.replace(/\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b/g, " ");
    for (const token of body.matchAll(/[A-Za-z_][A-Za-z0-9_-]*/g)) {
      const name = token[0];
      if (body.slice(0, token.index).trimEnd().endsWith(".")) continue;
      if (["true", "false", "null"].includes(name)) continue;
      if (
        body
          .slice(token.index + name.length)
          .trimStart()
          .startsWith("(")
      )
        check(
          [...FUNCTIONS, ...special].includes(name.toLowerCase()),
          `unavailable expression function ${name} at ${location}`,
        );
      else
        check(
          contexts.includes(name),
          `unavailable expression context ${name} at ${location}`,
        );
    }
    offset = end + 2;
  }
}
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
  expressions(w.env, [...GLOBAL, "secrets"], "env");
  expressions(w.concurrency, GLOBAL, "concurrency");
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
  for (const [id, job] of Object.entries(w.jobs)) {
    const location = `jobs.${id}`;
    expressions(job.env, [...JOB, "secrets"], `${location}.env`);
    for (const key of ["name", "runs-on", "timeout-minutes"])
      expressions(job[key], JOB, `${location}.${key}`);
    expressions(job.strategy, [...GLOBAL, "needs"], `${location}.strategy`);
    check(!job.permissions, "jobs cannot override read-only permissions");
    for (const [index, step] of job.steps.entries()) {
      const position = `${location}.steps.${index}`;
      for (const key of ["name", "env", "with", "working-directory"])
        expressions(step[key], [...STEP, "secrets"], `${position}.${key}`, [
          "hashfiles",
        ]);
      expressions(
        step.if,
        STEP,
        `${position}.if`,
        ["hashfiles", "always", "cancelled", "success", "failure"],
        true,
      );
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
    check(
      process.argv.length <= 3,
      "usage: check-workflow.mjs [workflow-file]",
    );
    validateWorkflow(process.argv[2]);
    console.log(
      "Candidate workflow syntax, bounded expression scopes and release contract passed.",
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
