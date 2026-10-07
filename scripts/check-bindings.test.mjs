import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { compareBindings } from "./check-bindings.mjs";

function fixture(t, expectedFiles, generatedFiles) {
  const root = mkdtempSync(join(tmpdir(), "windfall-ci-bindings-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const folders = { expected: expectedFiles, generated: generatedFiles };
  for (const [folder, files] of Object.entries(folders)) {
    mkdirSync(join(root, folder));
    for (const [name, content] of Object.entries(files)) {
      writeFileSync(join(root, folder, name), content);
    }
  }
  return compareBindings(join(root, "expected"), join(root, "generated"));
}

const type = { "Project.ts": "export type Project = { tempo: number };\n" };

test("identical types, barrel and fixtures pass with either checkout line ending", (t) => {
  const files = {
    ...type,
    "index.ts": 'export type { Project } from "./Project";\n',
    "descriptors.json": '{"gain": 0.5}\n',
  };
  const crlf = Object.fromEntries(
    Object.entries(files).map(([name, text]) => [name, text.replace(/\n/g, "\r\n")]),
  );
  assert.deepEqual(fixture(t, files, crlf), []);
});

test("new untracked exports fail", (t) => {
  assert.deepEqual(
    fixture(t, type, { ...type, "NewType.ts": "export type NewType = string;\n" }),
    ["new export: NewType.ts"],
  );
});

test("obsolete checked-in exports fail", (t) => {
  assert.deepEqual(
    fixture(t, { ...type, "OldType.ts": "export type OldType = string;\n" }, type),
    ["obsolete export: OldType.ts"],
  );
});

test("changed types and barrels fail", (t) => {
  assert.deepEqual(
    fixture(t, { ...type, "index.ts": "old\n" }, {
      "Project.ts": "export type Project = string;\n",
      "index.ts": "new\n",
    }),
    ["changed: Project.ts", "changed: index.ts"],
  );
});

test("JSON fixture changes fail without rounding away numeric differences", (t) => {
  assert.deepEqual(
    fixture(t, { ...type, "descriptors.json": '{"gain": 0.5}\n' }, {
      ...type,
      "descriptors.json": '{"gain": 0.5000001}\n',
    }),
    ["changed: descriptors.json"],
  );
});

function automation(shapes, rest = {}) {
  return { ...type, "automation-fixtures.json": JSON.stringify({ shapes, ...rest }) };
}

test("the actual Windows/Linux f64 shape sample differences pass", (t) => {
  const windows = [{ curve: -1, parts: [0.1, 0.3], values: [0.45230952713087613, 0.624523536256335] }];
  const linux = [{ curve: -1, parts: [0.1, 0.3], values: [0.4523095271308762, 0.6245235362563352] }];
  assert.deepEqual(fixture(t, automation(windows), automation(linux)), []);
});

test("shape drift beyond four representable f64 values fails", (t) => {
  const expected = [{ curve: -1, parts: [0.1], values: [0.5] }];
  const boundary = [{ curve: -1, parts: [0.1], values: [0.5 + 4 * 2 ** -53] }];
  const beyond = [{ curve: -1, parts: [0.1], values: [0.5 + 5 * 2 ** -53] }];
  assert.deepEqual(fixture(t, automation(expected), automation(boundary)), []);
  assert.deepEqual(fixture(t, automation(expected), automation(beyond)), ["changed: automation-fixtures.json"]);
});

test("shape input and sample-count changes fail even within the ULP allowance", (t) => {
  const expected = [{ curve: -1, parts: [0.5], values: [0.5] }];
  const input = [{ curve: -1, parts: [0.5 + 2 ** -53], values: [0.5] }];
  const fewer = [{ curve: -1, parts: [0.5], values: [] }];
  assert.deepEqual(fixture(t, automation(expected), automation(input)), ["changed: automation-fixtures.json"]);
  assert.deepEqual(fixture(t, automation(expected), automation(fewer)), ["changed: automation-fixtures.json"]);
});

test("automation f32 curves and range metadata still compare exactly", (t) => {
  const expected = automation([], { curves: [{ values: [0.5] }], ranges: [{ min: 0.5 }] });
  const curve = automation([], { curves: [{ values: [0.5 + 2 ** -53] }], ranges: [{ min: 0.5 }] });
  const range = automation([], { curves: [{ values: [0.5] }], ranges: [{ min: 0.5 + 2 ** -53 }] });
  assert.deepEqual(fixture(t, expected, curve), ["changed: automation-fixtures.json"]);
  assert.deepEqual(fixture(t, expected, range), ["changed: automation-fixtures.json"]);
});

test("empty generation fails even if the checked-in folder is also empty", (t) => {
  assert.deepEqual(fixture(t, {}, {}), ["no generated TypeScript types"]);
});
