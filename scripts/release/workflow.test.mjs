import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { validateWorkflow } from "./check-workflow.mjs";

const workflow = fileURLToPath(
  new URL("../../.github/workflows/release-candidate.yml", import.meta.url),
);
const validator = fileURLToPath(
  new URL("./check-workflow.mjs", import.meta.url),
);

function fixture(t, edit) {
  const parent = fs.realpathSync(os.tmpdir());
  const root = fs.mkdtempSync(path.join(parent, "windfall-r1-workflow-test-"));
  fs.writeFileSync(path.join(root, ".owner"), "release-workflow-test");
  t.after(() => {
    assert.equal(path.dirname(fs.realpathSync(root)), parent);
    assert.ok(path.basename(root).startsWith("windfall-r1-workflow-test-"));
    assert.equal(
      fs.readFileSync(path.join(root, ".owner"), "utf8"),
      "release-workflow-test",
    );
    fs.rmSync(root, { recursive: true });
  });
  const value = JSON.parse(fs.readFileSync(workflow, "utf8"));
  edit(value);
  const file = path.join(root, "candidate.yml");
  fs.writeFileSync(file, JSON.stringify(value, null, 2));
  return file;
}

test("checked workflow retains manual-only trigger, pins and candidate gates", () => {
  const value = validateWorkflow();
  assert.deepEqual(Object.keys(value.on), ["workflow_dispatch"]);
  assert.deepEqual(value.permissions, { contents: "read" });
});

for (const key of ["TS_RS_EXPORT_DIR", "CANDIDATE_OUTPUT", "SOURCE_CAPTURE"])
  test(`GitHub regression: runner context is unavailable in job env ${key}`, (t) => {
    const file = fixture(t, (value) => {
      value.jobs.candidate.env[key] = "${{ runner.temp }}/candidate";
    });
    assert.throws(
      () => validateWorkflow(file),
      /context.*runner.*jobs.candidate.env/,
    );
  });

for (const expression of [
  "${{ runner['temp'] }}",
  "${{ env.CANDIDATE_OUTPUT }}",
  "${{ steps.capture.outputs.path }}",
  "${{ invented.path }}",
])
  test(`job env rejects unavailable expression ${expression}`, (t) => {
    const file = fixture(t, (value) => {
      value.jobs.candidate.env.SOURCE_CAPTURE = expression;
    });
    assert.throws(() => validateWorkflow(file), /context/);
  });

test("global env cannot acquire a runner or matrix context", (t) => {
  for (const context of ["runner", "matrix"])
    assert.throws(
      () =>
        validateWorkflow(
          fixture(t, (value) => {
            value.env.EXTRA = `\u0024{{ ${context}.temp }}`;
          }),
        ),
      /context/,
    );
});

test("runner context and hashFiles are valid in step env", (t) => {
  const file = fixture(t, (value) => {
    value.jobs.candidate.steps.unshift({
      name: "Scope fixture",
      env: {
        PATH_EVIDENCE: "${{ runner.temp }}/candidate",
        LOCK_EVIDENCE: "${{ hashFiles('source/Cargo.lock') }}",
        LITERAL: "${{ format('runner.temp {0}', matrix.platform) }}",
      },
      run: "echo scope-fixture",
    });
  });
  assert.doesNotThrow(() => validateWorkflow(file));
});

test("job env rejects step-only hashFiles function", (t) => {
  const file = fixture(t, (value) => {
    value.jobs.candidate.env.EXTRA = "${{ hashFiles('source/Cargo.lock') }}";
  });
  assert.throws(() => validateWorkflow(file), /function.*hashFiles/);
});

test("unterminated expressions fail closed", (t) => {
  const file = fixture(t, (value) => {
    value.jobs.candidate.env.EXTRA = "${{ matrix.platform";
  });
  assert.throws(() => validateWorkflow(file), /unterminated expression/);
});

test("validator CLI exits nonzero for the actual GitHub context regression", (t) => {
  const file = fixture(t, (value) => {
    value.jobs.candidate.env.SOURCE_CAPTURE = "${{ runner.temp }}/capture.json";
  });
  const bad = spawnSync(process.execPath, [validator, file], {
    encoding: "utf8",
  });
  assert.equal(bad.status, 1);
  assert.match(bad.stderr, /context.*runner/);
  const good = spawnSync(process.execPath, [validator], { encoding: "utf8" });
  assert.equal(good.status, 0, good.stderr);
});

test("actual initialization command exports unchanged paths before their consumers", (t) => {
  const value = validateWorkflow();
  const steps = value.jobs.candidate.steps;
  const setup = steps.findIndex(
    (step) => step.name === "Resolve native runner paths for later steps",
  );
  assert.ok(setup >= 0);
  assert.ok(
    setup < steps.findIndex((step) => step.name.startsWith("Capture clean")),
  );
  assert.deepEqual(Object.keys(steps[setup].env).sort(), [
    "CANDIDATE_OUTPUT",
    "SOURCE_CAPTURE",
    "TS_RS_EXPORT_DIR",
  ]);
  const file = fixture(t, () => {});
  const output = path.join(path.dirname(file), "exported.env");
  const simulated = {
    TS_RS_EXPORT_DIR: "/runner temp/space's-$()/release-r1-bindings",
    CANDIDATE_OUTPUT: "/runner temp/space's-$()/windfall-candidate-windows-x64",
    SOURCE_CAPTURE: "/runner temp/space's-$()/windfall-source-capture.json",
  };
  const bash =
    process.platform === "win32"
      ? path.join(
          process.env.ProgramFiles ?? "C:/Program Files",
          "Git/bin/bash.exe",
        )
      : "bash";
  const result = spawnSync(
    bash,
    ["--noprofile", "--norc", "-c", steps[setup].run],
    {
      encoding: "utf8",
      env: { ...process.env, ...simulated, GITHUB_ENV: output },
    },
  );
  assert.equal(result.status, 0, result.stderr ?? result.error?.message);
  assert.equal(
    fs.readFileSync(output, "utf8"),
    Object.entries(simulated)
      .map(([key, entry]) => `${key}=${entry}\n`)
      .join(""),
  );
});

for (const [name, edit, error] of [
  [
    "automatic trigger",
    (value) => {
      value.on.push = {};
    },
    /workflow triggers/,
  ],
  [
    "write permission",
    (value) => {
      value.permissions.contents = "write";
    },
    /read-only/,
  ],
  [
    "floating action",
    (value) => {
      value.jobs.candidate.steps.find((s) => s.uses).uses =
        "actions/checkout@main";
    },
    /action pin/,
  ],
  [
    "implicit signing",
    (value) => {
      const s = value.jobs.candidate.steps.find((s) =>
        s.name.startsWith("Build native"),
      );
      s.run = s.run.replace("--no-sign", "");
    },
    /disable signing/,
  ],
  [
    "missing verification",
    (value) => {
      value.jobs.candidate.steps = value.jobs.candidate.steps.filter(
        (s) => s.name !== "Independent verification before upload",
      );
    },
    /verified no-clobber/,
  ],
  [
    "overwrite upload",
    (value) => {
      value.jobs.candidate.steps.find((s) =>
        s.uses?.startsWith("actions/upload-artifact@"),
      ).with.overwrite = true;
    },
    /verified no-clobber/,
  ],
])
  test(`retained release gate rejects ${name}`, (t) => {
    assert.throws(() => validateWorkflow(fixture(t, edit)), error);
  });
