#!/usr/bin/env node
// Compare a fresh generation with the checked-in bindings, including files
// git diff cannot see (new untracked exports) and obsolete checked-in types.
import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

function sameShapeSample(expected, actual) {
  if (expected === actual) return true;
  // curve_shape samples lie in [0, 1]. Windows and Unix exp implementations
  // differ by a few representable f64 values; compare their bit distance.
  if (!Number.isFinite(expected) || !Number.isFinite(actual) || expected < 0 || actual < 0) {
    return false;
  }
  const bits = new DataView(new ArrayBuffer(8));
  bits.setFloat64(0, expected);
  const expectedBits = bits.getBigUint64(0);
  bits.setFloat64(0, actual);
  const actualBits = bits.getBigUint64(0);
  const distance = expectedBits > actualBits ? expectedBits - actualBits : actualBits - expectedBits;
  return distance <= 4n;
}

function sameAutomationFixtures(expectedText, actualText) {
  const expected = JSON.parse(expectedText);
  const actual = JSON.parse(actualText);
  if (Array.isArray(expected.shapes) && Array.isArray(actual.shapes)) {
    actual.shapes.forEach((shape, index) => {
      const expectedShape = expected.shapes[index];
      if (!Array.isArray(shape?.values) || !Array.isArray(expectedShape?.values)) return;
      shape.values = shape.values.map((value, sample) =>
        sameShapeSample(expectedShape.values[sample], value) ? expectedShape.values[sample] : value,
      );
    });
  }
  // Only the computed f64 shape samples above are normalized. Curve inputs,
  // f32 outputs, ranges, descriptors and every type still compare exactly.
  return JSON.stringify(expected) === JSON.stringify(actual);
}

function filesBelow(folder, prefix = "") {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const name = `${prefix}${entry.name}`;
    return entry.isDirectory()
      ? filesBelow(join(folder, entry.name), `${name}/`)
      : [name];
  });
}

export function compareBindings(expected, generated) {
  const expectedFiles = new Set(filesBelow(expected));
  const generatedFiles = new Set(filesBelow(generated));
  const differences = [];
  if (![...generatedFiles].some((file) => file.endsWith(".ts") && file !== "index.ts")) {
    differences.push("no generated TypeScript types");
  }
  for (const file of [...new Set([...expectedFiles, ...generatedFiles])].sort()) {
    if (!expectedFiles.has(file)) {
      differences.push(`new export: ${file}`);
    } else if (!generatedFiles.has(file)) {
      differences.push(`obsolete export: ${file}`);
    } else {
      // Ignore checkout line endings; allow only the documented shape ULPs.
      const text = (folder) => readFileSync(join(folder, file), "utf8").replace(/\r\n/g, "\n");
      const expectedText = text(expected);
      const actualText = text(generated);
      if (
        expectedText !== actualText &&
        !(file === "automation-fixtures.json" && sameAutomationFixtures(expectedText, actualText))
      ) {
        differences.push(`changed: ${file}`);
      }
    }
  }
  return differences;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const generated = process.argv[2];
  if (!generated || process.argv.length !== 3) {
    console.error("usage: node scripts/check-bindings.mjs <generated-directory>");
    process.exit(1);
  }
  const expected = fileURLToPath(new URL("../apps/desktop/src/bindings", import.meta.url));
  const differences = compareBindings(expected, generated);
  if (differences.length) {
    for (const difference of differences) console.error(`check-bindings: ${difference}`);
    console.error("check-bindings: run scripts/gen-bindings.sh and commit all generated files.");
    process.exit(1);
  }
  console.log("check-bindings: all bindings and fixtures are current");
}
