#!/usr/bin/env node
// Validates docs/parity/parity.json and generates docs/parity/PARITY.md from it.
//
//   node scripts/parity.mjs           validate, regenerate PARITY.md, print a summary
//   node scripts/parity.mjs --check   validate and fail if PARITY.md is stale (for CI)
//
// No dependencies. parity.json is the source of truth; PARITY.md is never edited by hand.

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

const JSON_PATH = fileURLToPath(new URL("../docs/parity/parity.json", import.meta.url));
const MD_PATH = fileURLToPath(new URL("../docs/parity/PARITY.md", import.meta.url));

const AREAS = {
  core: "Core features",
  window: "Main windows",
  instrument: "Instruments",
  effect: "Effects",
  visual: "Visual and video",
  editor: "Audio editors",
  format: "File formats and plugin hosting",
  workflow: "Workflow, MIDI and settings",
};
const STATUSES = { todo: "Todo", "in-progress": "In progress", done: "Done", "wont-do": "Won't do" };
const EDITIONS = ["fruity", "producer", "signature", "all-plugins"];
const PHASES = [
  "Spike", "Make a beat", "Write a song", "Record and edit audio",
  "Plugins and files", "The long tail", "Extras", "Release",
];
// Areas whose rows are FL plugins. Their Windfall name must never be the FL name.
const PLUGIN_AREAS = ["instrument", "effect", "visual", "editor"];
const REQUIRED_KEYS = ["id", "area", "fl_feature", "summary", "windfall_name", "phase", "status", "reason", "fl_edition"];
const OPTIONAL_KEYS = ["notes"];

const isText = (v) => typeof v === "string" && v.trim() !== "";
const isObject = (v) => v !== null && typeof v === "object" && !Array.isArray(v);

function validate(doc) {
  const errors = [];
  if (!isObject(doc)) return ["parity.json must be an object"];

  if (!Array.isArray(doc.generated_from) || doc.generated_from.length === 0 || !doc.generated_from.every(isText)) {
    errors.push("generated_from must be a non-empty array of source URLs");
  }
  if (typeof doc.as_of !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(doc.as_of)) {
    errors.push("as_of must be a YYYY-MM-DD date string");
  }
  if (!Array.isArray(doc.rows) || doc.rows.length === 0) {
    errors.push("rows must be a non-empty array");
    return errors;
  }

  const seen = new Map();
  doc.rows.forEach((row, index) => {
    const where = `rows[${index}]${isObject(row) && isText(row.id) ? ` (${row.id})` : ""}`;
    const fail = (message) => errors.push(`${where}: ${message}`);
    if (!isObject(row)) return fail("must be an object");

    for (const key of REQUIRED_KEYS) if (!(key in row)) fail(`missing field "${key}"`);
    for (const key of Object.keys(row)) {
      if (!REQUIRED_KEYS.includes(key) && !OPTIONAL_KEYS.includes(key)) fail(`unknown field "${key}"`);
    }

    if (!isText(row.id) || !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(row.id)) {
      fail("id must be a kebab-case slug");
    } else if (seen.has(row.id)) {
      fail(`duplicate id, first used at rows[${seen.get(row.id)}]`);
    } else {
      seen.set(row.id, index);
    }

    if (!Object.hasOwn(AREAS, row.area)) fail(`area must be one of ${Object.keys(AREAS).join(", ")}`);
    if (!isText(row.fl_feature)) fail("fl_feature must be a non-empty string");
    if (!isText(row.summary)) fail("summary must be a non-empty string");
    if (row.windfall_name !== null && !isText(row.windfall_name)) fail("windfall_name must be null or a non-empty string");
    if (
      isText(row.windfall_name) && isText(row.fl_feature) && PLUGIN_AREAS.includes(row.area) &&
      row.windfall_name.trim().toLowerCase() === row.fl_feature.trim().toLowerCase()
    ) {
      fail("windfall_name must not reuse the FL plugin name");
    }
    if (!Number.isInteger(row.phase) || row.phase < 0 || row.phase > 7) fail("phase must be an integer from 0 to 7");
    if (!Object.hasOwn(STATUSES, row.status)) fail(`status must be one of ${Object.keys(STATUSES).join(", ")}`);
    if (row.status === "wont-do") {
      if (!isText(row.reason)) fail('reason is required when status is "wont-do"');
    } else if (row.reason !== null) {
      fail('reason must be null unless status is "wont-do"');
    }
    if (row.fl_edition !== null && !EDITIONS.includes(row.fl_edition)) fail(`fl_edition must be null or one of ${EDITIONS.join(", ")}`);
    if ("notes" in row && row.notes !== null && !isText(row.notes)) fail("notes must be null or a non-empty string");
  });
  return errors;
}

function countStatuses(rows) {
  const counts = Object.fromEntries(Object.keys(STATUSES).map((status) => [status, 0]));
  for (const row of rows) counts[row.status] += 1;
  return counts;
}

function summarize(rows) {
  const counts = countStatuses(rows);
  const total = rows.length;
  const accounted = counts.done + counts["wont-do"];
  const percent = total === 0 ? "0.0" : ((accounted / total) * 100).toFixed(1);
  return { counts, total, accounted, percent };
}

const cell = (value) => String(value).replace(/\s+/g, " ").replace(/\|/g, "\\|").trim();
const table = (header, rows) => [
  `| ${header.join(" | ")} |`,
  `| ${header.map(() => "---").join(" | ")} |`,
  ...rows.map((row) => `| ${row.map(cell).join(" | ")} |`),
];

function render(doc) {
  const { counts, total, accounted, percent } = summarize(doc.rows);
  const statusKeys = Object.keys(STATUSES);
  const lines = [
    "# Windfall parity matrix",
    "",
    "> **Generated file.** Do not edit it. Edit `docs/parity/parity.json` and run `node scripts/parity.mjs`.",
    ">",
    '> The "FL feature" column lists FL Studio feature and plugin names as plain references only ("FL equivalent").',
    "> Notes may mention FL names for the same purpose. They are Image-Line's names and are never used as Windfall",
    "> names. `TBD` in the Windfall column means the Windfall name has not been chosen yet.",
    "",
    `As of ${doc.as_of}. Sources:`,
    "",
    ...doc.generated_from.map((url) => `- <${url}>`),
    "",
    "## Summary",
    "",
    `**${accounted} of ${total} rows accounted for (${percent}%).** A row is accounted for when it is done or won't do.`,
    "",
    ...table(["Status", "Rows"], statusKeys.map((status) => [STATUSES[status], counts[status]])),
    "",
    ...table(
      ["Area", "Rows", ...statusKeys.map((status) => STATUSES[status])],
      Object.keys(AREAS).map((area) => {
        const areaRows = doc.rows.filter((row) => row.area === area);
        const areaCounts = countStatuses(areaRows);
        return [AREAS[area], areaRows.length, ...statusKeys.map((status) => areaCounts[status])];
      }),
    ),
    "",
    ...table(
      ["Phase", "Rows", ...statusKeys.map((status) => STATUSES[status])],
      PHASES.map((name, phase) => {
        const phaseRows = doc.rows.filter((row) => row.phase === phase);
        const phaseCounts = countStatuses(phaseRows);
        return [`${phase}. ${name}`, phaseRows.length, ...statusKeys.map((status) => phaseCounts[status])];
      }),
    ),
  ];

  for (const area of Object.keys(AREAS)) {
    const areaRows = doc.rows.filter((row) => row.area === area);
    if (areaRows.length === 0) continue;
    lines.push(
      "",
      `## ${AREAS[area]}`,
      "",
      ...table(
        ["FL feature", "Windfall", "Phase", "Status", "Notes"],
        areaRows.map((row) => {
          const notes = [row.status === "wont-do" ? `Reason: ${row.reason}` : null, row.notes ?? null].filter(Boolean);
          return [row.fl_feature, row.windfall_name ?? "TBD", row.phase, row.status, notes.join(" ")];
        }),
      ),
    );
  }
  return lines.join("\n") + "\n";
}

function main() {
  const args = process.argv.slice(2);
  const unknown = args.filter((arg) => arg !== "--check");
  if (unknown.length > 0) {
    console.error(`parity: unknown argument ${unknown.join(" ")}\nusage: node scripts/parity.mjs [--check]`);
    return 2;
  }
  const check = args.includes("--check");

  let doc;
  try {
    doc = JSON.parse(readFileSync(JSON_PATH, "utf8"));
  } catch (error) {
    console.error(`parity: cannot read ${JSON_PATH}: ${error.message}`);
    return 1;
  }

  const errors = validate(doc);
  if (errors.length > 0) {
    console.error(`parity: parity.json is invalid (${errors.length} problem${errors.length === 1 ? "" : "s"})`);
    for (const error of errors) console.error(`  - ${error}`);
    return 1;
  }

  const markdown = render(doc);
  // Compare with line endings normalized so a CRLF checkout does not read as stale.
  const current = existsSync(MD_PATH) ? readFileSync(MD_PATH, "utf8").replace(/\r\n/g, "\n") : null;
  const stale = current !== markdown;

  const { counts, total, accounted, percent } = summarize(doc.rows);
  console.log(`parity: ${total} rows`);
  console.log(`  ${Object.keys(STATUSES).map((status) => `${status} ${counts[status]}`).join(", ")}`);
  console.log(`  accounted for: ${accounted}/${total} (${percent}%)`);

  if (check) {
    if (stale) {
      console.error("parity: docs/parity/PARITY.md is stale. Run `node scripts/parity.mjs` and commit the result.");
      return 1;
    }
    console.log("  PARITY.md is up to date");
    return 0;
  }

  if (stale) {
    writeFileSync(MD_PATH, markdown);
    console.log("  wrote docs/parity/PARITY.md");
  } else {
    console.log("  PARITY.md already up to date");
  }
  return 0;
}

process.exitCode = main();
