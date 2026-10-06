// Drives apps/desktop/bench.html in installed Microsoft Edge and collects
// window.__benchResults for every configuration.
//
// playwright-core is not a dependency of the app. Install it in any scratch
// folder and run this script from there:
//
//   mkdir bench-runner && cd bench-runner && pnpm init && pnpm add playwright-core
//   (in apps/desktop)  pnpm exec vite --port 1433 --strictPort
//   node <repo>/docs/perf/run-canvas-bench.mjs --out results.json
//
// Options (all optional):
//   --base-url   http://localhost:1433
//   --notes      10000,50000
//   --renderers  canvas2d,webgl2,webgpu
//   --dpr        1            1 runs in a real full-screen window. Other
//                             values use device metrics emulation.
//   --seconds    4            measured seconds per scenario
//   --gpu-seconds 1.5         seconds per GPU timing pass, 0 to skip
//   --scenarios  scroll,zoom  default is all
//   --uncapped                disable vsync and the frame rate limit
//   --browser-args=<flags>    extra Edge switches, space separated, e.g.
//                             --browser-args="--disable-gpu" to measure
//                             software rendering
//   --headless                no window (software rendering is likely)
//   --cdp <url>               attach to a running web view instead of
//                             launching Edge. See webview2-host.ps1.
//   --label <text>            stored with the results
//   --screenshots <dir>       save dark and light screenshots and exit
//   --out        <file>       write the JSON results here

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs"
import { createRequire } from "node:module"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"

const require = createRequire(join(process.cwd(), "index.js"))
const { chromium } = require("playwright-core")

function parseArgs(argv) {
  const args = {}
  for (let i = 0; i < argv.length; i++) {
    const key = argv[i]
    if (!key.startsWith("--")) continue
    // --name=value, for values that themselves start with dashes.
    const equals = key.indexOf("=")
    if (equals > 0) {
      args[key.slice(2, equals)] = key.slice(equals + 1)
      continue
    }
    const next = argv[i + 1]
    if (next === undefined || next.startsWith("--")) args[key.slice(2)] = true
    else args[key.slice(2)] = argv[++i]
  }
  return args
}

const args = parseArgs(process.argv.slice(2))
const baseUrl = args["base-url"] ?? "http://localhost:1433"
const noteCounts = String(args.notes ?? "10000,50000").split(",").map(Number)
const renderers = String(args.renderers ?? "canvas2d,webgl2,webgpu").split(",")
const dprs = String(args.dpr ?? "1").split(",").map(Number)
const seconds = Number(args.seconds ?? 4)
const gpuSeconds = Number(args["gpu-seconds"] ?? 1.5)
const uncapped = args.uncapped === true
const headless = args.headless === true
const cdpUrl = typeof args.cdp === "string" ? args.cdp : null
const WIDTH = 1920
const HEIGHT = 1080

function launchArgs(dpr) {
  const list = [
    // A covered or unfocused window must keep animating at full rate.
    "--disable-backgrounding-occluded-windows",
    "--disable-renderer-backgrounding",
    "--disable-background-timer-throttling",
    "--no-first-run",
    "--no-default-browser-check",
    "--hide-crash-restore-bubble",
  ]
  // A real 1920x1080 viewport needs the window to cover the whole screen.
  if (!headless) list.push("--kiosk", "--window-position=0,0")
  if (uncapped) list.push("--disable-gpu-vsync", "--disable-frame-rate-limit")
  if (typeof args["browser-args"] === "string") {
    list.push(...args["browser-args"].split(" ").filter((flag) => flag !== ""))
  }
  return list
}

async function openBrowser(dpr) {
  if (cdpUrl) {
    // The host owns the window and its size. Nothing is emulated.
    const browser = await chromium.connectOverCDP(cdpUrl)
    const context = browser.contexts()[0]
    const page = context.pages()[0] ?? (await context.newPage())
    return { context, page, profile: null, browser }
  }
  const real = dpr === 1 && !headless
  // A persistent context is the only way to drive the window that the
  // launch flags apply to. A normal context opens a second, smaller one.
  const profile = mkdtempSync(join(tmpdir(), "wf-bench-profile-"))
  const context = await chromium.launchPersistentContext(profile, {
    channel: "msedge",
    headless,
    args: launchArgs(dpr),
    ...(real
      ? { viewport: null }
      : { viewport: { width: WIDTH, height: HEIGHT }, deviceScaleFactor: dpr }),
  })
  const page = context.pages()[0] ?? (await context.newPage())
  return { context, page, profile, browser: context.browser() }
}

async function closeBrowser({ context, profile, browser }) {
  if (cdpUrl) {
    // Detaches. The host process keeps running.
    await browser.close()
    return
  }
  await context.close()
  try {
    rmSync(profile, { recursive: true, force: true, maxRetries: 5 })
  } catch {
    // Edge can hold a file in the profile for a moment after exit.
  }
}

/** Reads the feature status and driver tables from edge://gpu. */
async function readGpuPage(page) {
  try {
    await page.goto("edge://gpu", { waitUntil: "load" })
    await page.waitForTimeout(1500)
    const lines = await page.evaluate(() => {
      const view = document.querySelector("info-view")
      const root = view?.shadowRoot ?? document
      const items = [...root.querySelectorAll("li")].map((li) =>
        li.innerText.trim()
      )
      const rows = [...root.querySelectorAll("tr")].map((tr) =>
        [...tr.cells].map((cell) => cell.innerText.trim()).join(": ")
      )
      return [...items, ...rows]
    })
    const pick = (label) => {
      const line = lines.find((l) => l.startsWith(`${label}:`))
      return line ? line.slice(label.length + 1).trim() : null
    }
    const features = {}
    for (const name of [
      "Canvas",
      "Compositing",
      "Rasterization",
      "WebGL",
      "WebGPU",
      "Skia Graphite",
      "Multiple Raster Threads",
    ]) {
      features[name] = pick(name)
    }
    return {
      features,
      glRenderer: pick("GL_RENDERER"),
      displayType: pick("Display type"),
      commandLine: pick("Command Line"),
      lines: lines.slice(0, 200),
    }
  } catch (error) {
    return { error: String(error) }
  }
}

function benchUrl(params) {
  const query = new URLSearchParams(params)
  return `${baseUrl}/bench.html?${query}`
}

async function runOne(page, config) {
  const errors = []
  const onPageError = (error) => errors.push(String(error))
  const onConsole = (message) => {
    if (message.type() === "error") errors.push(message.text())
  }
  page.on("pageerror", onPageError)
  page.on("console", onConsole)
  const params = {
    notes: String(config.notes),
    renderer: config.renderer,
    theme: "dark",
    auto: "1",
    seconds: String(seconds),
    gpuSeconds: String(gpuSeconds),
  }
  if (typeof args.scenarios === "string") params.scenarios = args.scenarios
  await page.goto(benchUrl(params))
  await page.bringToFront()
  await page.waitForFunction(() => window.__benchDone === true, null, {
    timeout: 300_000,
  })
  const outcome = await page.evaluate(() => ({
    results: window.__benchResults ?? null,
    error: window.__benchError ?? null,
  }))
  page.off("pageerror", onPageError)
  page.off("console", onConsole)
  // Unload the page so the next configuration starts from a clean document.
  await page.goto("about:blank")
  return { ...config, ...outcome, pageErrors: errors }
}

async function screenshots(dir) {
  mkdirSync(dir, { recursive: true })
  for (const dpr of dprs) {
    const session = await openBrowser(dpr)
    const { page } = session
    const suffix = dpr === 1 ? "" : `@${dpr}x`
    for (const theme of ["dark", "light"]) {
      await page.goto(
        benchUrl({ notes: "10000", renderer: renderers[0], theme })
      )
      await page.waitForTimeout(1200)
      await page.screenshot({ path: join(dir, `bench-${theme}${suffix}.png`) })

      // A marquee held open over part of the view, with the playhead running.
      await page.mouse.move(420, 300)
      await page.mouse.down()
      await page.mouse.move(980, 640, { steps: 8 })
      await page.keyboard.press("Space")
      await page.waitForTimeout(900)
      await page.screenshot({
        path: join(dir, `bench-${theme}-selection${suffix}.png`),
      })
      await page.screenshot({
        path: join(dir, `bench-${theme}-detail${suffix}.png`),
        clip: { x: 380, y: 260, width: 480, height: 270 },
      })
      await page.mouse.up()
    }
    await closeBrowser(session)
  }
}

function fmt(value, digits = 2) {
  return typeof value === "number" ? value.toFixed(digits) : "n/a"
}

function printTable(runs) {
  console.log(
    "dpr | notes | renderer | scenario | fps | frame avg | p95 | p99 | max | >16.7ms % | missed % | update cpu avg/p95 | draw cpu avg/p95 | gpu avg/p95 | draw+finish avg | items"
  )
  for (const run of runs) {
    if (!run.results) {
      console.log(
        `${run.dpr} | ${run.notes} | ${run.renderer} | FAILED: ${run.error}`
      )
      continue
    }
    for (const s of run.results.scenarios) {
      console.log(
        [
          run.dpr,
          run.notes,
          run.renderer,
          s.name,
          fmt(s.fps, 1),
          fmt(s.frameMs.avg),
          fmt(s.frameMs.p95),
          fmt(s.frameMs.p99),
          fmt(s.frameMs.max),
          fmt(s.overBudgetPct, 1),
          fmt(s.missedFramePct, 1),
          `${fmt(s.updateCpuMs.avg)}/${fmt(s.updateCpuMs.p95)}`,
          `${fmt(s.drawCpuMs.avg)}/${fmt(s.drawCpuMs.p95)}`,
          s.gpuMs ? `${fmt(s.gpuMs.avg)}/${fmt(s.gpuMs.p95)}` : "n/a",
          s.drawAndFinishMs ? fmt(s.drawAndFinishMs.avg) : "n/a",
          fmt(s.itemsInRange, 0),
        ].join(" | ")
      )
    }
  }
}

async function main() {
  if (typeof args.screenshots === "string") {
    await screenshots(resolve(args.screenshots))
    return
  }

  const runs = []
  const environments = []
  for (const dpr of dprs) {
    const session = await openBrowser(dpr)
    const { page } = session
    environments.push({
      dpr,
      uncapped,
      headless,
      label: typeof args.label === "string" ? args.label : null,
      attachedTo: cdpUrl,
      browserVersion: session.browser?.version() ?? null,
      launchArgs: cdpUrl ? null : launchArgs(dpr),
      gpu: await readGpuPage(page),
    })
    // One throwaway load, so the first measured run does not pay for a
    // cold browser: GPU process start-up, shader cache, font loading.
    await page.goto(benchUrl({ notes: "10000", renderer: renderers[0] }))
    await page.waitForTimeout(3000)
    for (const notes of noteCounts) {
      for (const renderer of renderers) {
        process.stderr.write(`dpr ${dpr}  ${notes} notes  ${renderer} ... `)
        const run = await runOne(page, { dpr, notes, renderer, uncapped })
        process.stderr.write(run.results ? "ok\n" : `failed: ${run.error}\n`)
        runs.push(run)
      }
    }
    await closeBrowser(session)
  }

  printTable(runs)
  if (typeof args.out === "string") {
    const file = resolve(args.out)
    mkdirSync(dirname(file), { recursive: true })
    writeFileSync(
      file,
      JSON.stringify(
        { createdAt: new Date().toISOString(), environments, runs },
        null,
        2
      )
    )
    console.log(`wrote ${file}`)
  }
}

await main()
