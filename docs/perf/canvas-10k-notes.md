# Can the web view draw a 10,000-note piano roll at 60 fps?

Measured 2026-10-06 for the phase 0 spike. The renderer measured here is `apps/desktop/src/lib/canvas`. The test page is `apps/desktop/bench.html`.

## The question

`WINDFALL_PLAN.md` lists this as the first risk: "The web view cannot draw the editors fast enough", with the response "Test with 10,000 notes in phase 0. Move drawing to a GPU canvas early. Fall back to Electron if the system web view is the limit."

The question is whether a web view can draw a piano roll with 10,000 notes, and 50,000 as a stress case, at 60 fps while scrolling, zooming, dragging 1,000 selected notes and moving a playhead, and which drawing technique it takes.

## Answer

Yes on Windows, with a large margin, in both Edge and the real WebView2 control.

- **WebGL2 instanced quads** hold 60 fps in every scenario at 10,000 and 50,000 notes. With all 10,000 notes on screen a frame costs about 0.1 ms of main-thread time and 0.6 ms of GPU time, out of a 16.7 ms budget. With vsync off the same scene runs at about 4,000 frames per second. It still holds 60 fps with 1,000,000 notes on screen.
- **WebGPU** performs the same as WebGL2 and draws identical pixels. It buys nothing at these sizes.
- **Canvas 2D** passes at 10,000 notes with roughly 5 times headroom. At 50,000 notes all on screen it is at the limit. It missed 2.1% and 2.6% of frames in two runs and none in a third. At 100,000 it runs at 32 fps.
- **Without a GPU the order flips.** With hardware acceleration disabled, Canvas 2D still holds 60 fps at 10,000 notes, while WebGL2 on the software rasterizer drops to 12 fps.

The piano roll and playlist should use WebGL2 and fall back to Canvas 2D when there is no hardware WebGL. `createRenderer("auto")` does exactly that.

The headline numbers, every note on screen at once (the `overview` scenario), 1920×1080 at device pixel ratio 1, Edge:

| Renderer | Notes | Frame avg at 60 Hz | Missed frames | Draw CPU | GPU | Rate with vsync off |
|---|---|---|---|---|---|---|
| WebGL2 | 10,000 | 16.67 ms | 0% | 0.09 ms | 0.59 ms | 4,031 fps |
| WebGL2 | 50,000 | 16.67 ms | 0% | 0.14 ms | 0.94 ms | 3,640 fps |
| WebGPU | 10,000 | 16.67 ms | 0% | 0.16 ms | 0.07 ms | 4,035 fps |
| WebGPU | 50,000 | 16.67 ms | 0% | 0.14 ms | 0.88 ms | 3,985 fps |
| Canvas 2D | 10,000 | 16.67 ms | 0% | 1.26 ms | not measurable | 308 fps |
| Canvas 2D | 50,000 | 17.09 ms | 2.1% | 8.69 ms | not measurable | 64 fps |

Read the limits before trusting this beyond Windows. One machine was measured, and it is a fast one. macOS WKWebView and Linux WebKitGTK were not measured at all.

## Method

### Machine

| | |
|---|---|
| CPU | Intel Core i9-14900F, 24 cores, 32 threads |
| Memory | 32 GB |
| GPU | NVIDIA GeForce RTX 4070 SUPER, driver 32.0.16.1047 |
| Display | 1920×1080 at 60 Hz, 100% scaling |
| OS | Windows 11 Home, build 26300 |
| Browser | Microsoft Edge 154.0.4258.53 |
| Web view | Microsoft Edge WebView2 Runtime 154.0.4258.53 |

The machine was not idle. Other agents, two editors and a browser were running, at about 8% total CPU when checked. The numbers include that noise.

### Hardware acceleration

Confirmed, three ways.

- `edge://gpu` reported Canvas, Compositing, Rasterization, WebGL and WebGPU all "Hardware accelerated". Skia Graphite was disabled, so Canvas 2D ran on Skia's older GPU backend, Ganesh.
- WebGL's `UNMASKED_RENDERER_WEBGL` was `ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 SUPER (0x00002783) Direct3D11 vs_5_0 ps_5_0, D3D11)`.
- The WebGPU adapter reported `nvidia lovelace`.

The WebView2 runs reported the same three things.

### How the browser was run

Playwright (`playwright-core` 1.63.0) drove installed Edge, headed, in `--kiosk` mode, so the page really was 1920×1080 CSS pixels at device pixel ratio 1 and `requestAnimationFrame` ran at the display's 60 Hz. The script is `docs/perf/run-canvas-bench.mjs`.

Three flags stopped Edge from throttling a covered window (`--disable-backgrounding-occluded-windows`, `--disable-renderer-backgrounding`, `--disable-background-timer-throttling`). Playwright adds its own flags, including `--disable-field-trial-config` and `--no-sandbox`. The full command line is in the results file.

The WebView2 runs did not use Playwright's launcher. `docs/perf/webview2-host.ps1` opens one borderless WebView2 control over the whole screen (a WinForms host, `msedgewebview2.exe` with `--embedded-browser-webview=1`), and the script attached to it over the debugging port. That is the same runtime Tauri uses on Windows, but it is not the Tauri shell.

Device pixel ratio 2 was emulated, because the display is 1920×1080. The canvas backing store was a real 3840×2160 and the GPU drew all of it, but the window on screen stayed small, so the final composite to the display covered fewer pixels than a real 4K display would.

### What was drawn

A piano roll filling the window: 128 rows, black-key rows shaded, bar, beat and step lines, notes colored by velocity with a one-pixel border, selected notes in the selection colors, a marquee and a playhead. Bar numbers and octave names were drawn as text on the overlay canvas, 15 to 35 labels depending on zoom. Note names inside notes were not drawn.

The notes come from a seeded generator (`src/features/bench/generate-notes.ts`): a bass line, held chords, a melody, sixteenth-note arpeggios and scattered notes, over 200 bars and the 88 piano keys. Density is uniform, so 10,000 notes is 50 per bar and 50,000 is 250 per bar.

### Scenarios

Each ran for 4 seconds after a 0.4 second warm-up. There was one page load per renderer and note count, after one throwaway load to warm the browser up.

| Scenario | What changes every frame |
|---|---|
| `scroll` | Horizontal scroll through the whole song and back. 8 bars across the view, 16 px rows. |
| `zoom` | Zoom from all 200 bars across the view to 2 bars and back. |
| `drag` | 1,000 selected notes dragged in time and pitch. The move is a drag offset on the view. No note data changes. |
| `edit-rebuild` | The worst case for an edit. 1,000 notes change in the model, the whole batch is rebuilt from the `Note[]` array (colors, sort, spatial index) and uploaded again, every frame. |
| `playhead` | Only the playhead moves, at 140 BPM. The grid and notes are not redrawn. |
| `overview` | Every note on screen at once: the whole song across the view, all 88 keys, with a small zoom and scroll wobble so each frame is a full redraw. |

"Notes in range" in the tables is the size of the index range handed to the renderer per frame, after culling by time. Rows are clipped by the GPU or skipped in the Canvas 2D loop.

### How frame time was measured

Frame time is the difference between consecutive `requestAnimationFrame` timestamps. This is the ground truth. If the main thread, the compositor or the GPU falls behind, the browser skips a display refresh and the interval doubles.

- **Frame avg, p95, p99, max** are over about 240 frames per cell.
- **Missed** is the share of intervals longer than 1.5 refresh intervals (25 ms). This is the dropped-frame count.
- **Over 16.7 ms** is the share of intervals strictly above 16.7 ms, as the task asked. On a 60 Hz display it says almost nothing. The true interval is 16.67 ms and the timestamps jitter by a few tenths of a millisecond either side, so a third to a half of perfectly on-time frames read as 16.8. Use the max and Missed columns to judge.
- **Draw CPU** is `performance.now()` around the draw call (`view.flush()`): grid build, culling, uniforms and the draw calls, or for Canvas 2D the `fillRect` loop.
- **Update CPU** is the scenario's own work before drawing. It is near zero except in `edit-rebuild`, where it is the rebuild.

`performance.now()` and the frame timestamps have 0.1 ms resolution here (the page is not cross-origin isolated). Averages below 0.1 ms are averages of zeros and 0.1s.

### How GPU time was measured

Main-thread time does not show what the GPU did, so GPU time was measured separately, in its own 1.5 second pass per scenario so the measurement could not disturb the frame times.

- **WebGL2** used `EXT_disjoint_timer_query_webgl2`, which was available. One `TIME_ELAPSED` query wraps each frame's clear and draws. A second pass timed the draw call plus `gl.finish()`. It agreed with the main-thread numbers and is in the results file as `drawAndFinishMs`.
- **WebGPU** used the `timestamp-query` feature on the render pass. Chromium rounds these to 0.1 ms.
- **Canvas 2D** has no honest GPU number. Its drawing commands are recorded on the main thread and rasterized later in the GPU process. The only way to wait for that is a pixel readback, and repeated readbacks can make Chromium move the canvas to the CPU, which would change what is being measured. So for Canvas 2D the evidence is the frame intervals and the vsync-off rate.

GPU times from the 60 Hz runs are pessimistic. A GPU that works for half a millisecond every 16 ms drops to a low power state between frames. The same draws measured under continuous load, in the vsync-off runs, take about a tenth of the time. Both are in the tables.

### Vsync off

At 60 Hz a frame that costs 1 ms and a frame that costs 15 ms look the same. To see the real cost, a second set of runs started Edge with `--disable-gpu-vsync --disable-frame-rate-limit`, which lets `requestAnimationFrame` run as fast as the whole pipeline allows. The frame interval is then the true end-to-end cost of a frame, for all three renderers.

Every vsync-off run shows a few frames of 17 to 19 ms. They appear in all renderers and in the `playhead` scenario, which draws almost nothing, so they come from the browser's presentation path and not from the renderer.

## Results

All tables are generated from `docs/perf/canvas-10k-notes.results.json`. Times are milliseconds.

### Edge, 1920×1080, device pixel ratio 1, 60 Hz

| Notes | Renderer | Scenario | Notes in range | Frame avg | p95 | p99 | max | Over 16.7 ms | Missed | Update CPU | Draw CPU avg / p95 | GPU avg / p95 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 10,000 | canvas2d | scroll | 447 | 16.67 | 16.8 | 16.8 | 16.9 | 29% | 0.0% | 0.02 | 0.21 / 0.3 | n/a |
| 10,000 | canvas2d | zoom | 2,882 | 16.67 | 16.8 | 16.8 | 16.8 | 30% | 0.0% | 0.01 | 0.48 / 1.2 | n/a |
| 10,000 | canvas2d | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 35% | 0.0% | 0.01 | 0.32 / 0.4 | n/a |
| 10,000 | canvas2d | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.8 | 39% | 0.0% | 1.40 | 0.53 / 0.6 | n/a |
| 10,000 | canvas2d | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 44% | 0.0% | 0.01 | 0.06 / 0.1 | n/a |
| 10,000 | canvas2d | overview | 10,000 | 16.67 | 16.8 | 16.8 | 16.8 | 46% | 0.0% | 0.01 | 1.26 / 1.4 | n/a |
| 10,000 | webgl2 | scroll | 446 | 16.67 | 16.8 | 16.8 | 16.8 | 29% | 0.0% | 0.00 | 0.11 / 0.2 | 0.37 / 0.39 |
| 10,000 | webgl2 | zoom | 2,883 | 16.67 | 16.8 | 16.8 | 16.8 | 45% | 0.0% | 0.01 | 0.12 / 0.2 | 0.40 / 0.50 |
| 10,000 | webgl2 | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.9 | 49% | 0.0% | 0.01 | 0.05 / 0.1 | 0.46 / 1.63 |
| 10,000 | webgl2 | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.8 | 45% | 0.0% | 1.46 | 0.08 / 0.2 | 1.10 / 2.59 |
| 10,000 | webgl2 | playhead | 0 | 16.74 | 16.8 | 16.8 | 32.1 | 41% | 0.4% | 0.00 | 0.06 / 0.1 | n/a |
| 10,000 | webgl2 | overview | 10,000 | 16.67 | 16.8 | 17.0 | 17.2 | 34% | 0.0% | 0.00 | 0.09 / 0.2 | 0.59 / 0.57 |
| 10,000 | webgpu | scroll | 447 | 16.67 | 16.8 | 16.8 | 26.1 | 22% | 0.4% | 0.01 | 0.13 / 0.2 | 0.34 / 0.39 |
| 10,000 | webgpu | zoom | 2,912 | 16.67 | 16.8 | 16.8 | 16.8 | 41% | 0.0% | 0.01 | 0.12 / 0.2 | 0.40 / 0.52 |
| 10,000 | webgpu | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.9 | 46% | 0.0% | 0.01 | 0.09 / 0.2 | 0.38 / 0.39 |
| 10,000 | webgpu | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.9 | 17.3 | 51% | 0.0% | 1.41 | 0.11 / 0.2 | 0.38 / 0.39 |
| 10,000 | webgpu | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 56% | 0.0% | 0.01 | 0.06 / 0.1 | n/a |
| 10,000 | webgpu | overview | 10,000 | 16.67 | 16.8 | 16.8 | 17.1 | 30% | 0.0% | 0.01 | 0.16 / 0.3 | 0.07 / 0.20 |
| 50,000 | canvas2d | scroll | 2,261 | 16.67 | 16.8 | 16.8 | 16.9 | 31% | 0.0% | 0.01 | 0.43 / 0.7 | n/a |
| 50,000 | canvas2d | zoom | 14,596 | 16.67 | 16.8 | 16.8 | 17.1 | 29% | 0.0% | 0.01 | 2.26 / 6.8 | n/a |
| 50,000 | canvas2d | drag | 8,913 | 16.67 | 16.8 | 16.8 | 17.1 | 34% | 0.0% | 0.01 | 1.21 / 1.3 | n/a |
| 50,000 | canvas2d | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.8 | 16.9 | 38% | 0.0% | 6.82 | 2.19 / 2.3 | n/a |
| 50,000 | canvas2d | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 44% | 0.0% | 0.00 | 0.06 / 0.1 | n/a |
| 50,000 | canvas2d | overview | 50,000 | 17.09 | 16.8 | 33.3 | 50.0 | 47% | 2.1% | 0.02 | 8.69 / 9.8 | n/a |
| 50,000 | webgl2 | scroll | 2,258 | 16.67 | 16.7 | 16.8 | 16.8 | 28% | 0.0% | 0.01 | 0.14 / 0.3 | 0.57 / 0.58 |
| 50,000 | webgl2 | zoom | 14,449 | 16.67 | 16.8 | 16.8 | 16.9 | 39% | 0.0% | 0.01 | 0.12 / 0.2 | 0.69 / 1.09 |
| 50,000 | webgl2 | drag | 8,913 | 16.67 | 16.8 | 16.8 | 16.8 | 53% | 0.0% | 0.01 | 0.04 / 0.1 | 0.72 / 0.71 |
| 50,000 | webgl2 | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.8 | 16.9 | 53% | 0.0% | 7.03 | 0.15 / 0.2 | 0.64 / 0.65 |
| 50,000 | webgl2 | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 43% | 0.0% | 0.01 | 0.06 / 0.1 | n/a |
| 50,000 | webgl2 | overview | 50,000 | 16.67 | 16.8 | 16.8 | 16.8 | 25% | 0.0% | 0.01 | 0.14 / 0.2 | 0.94 / 1.07 |
| 50,000 | webgpu | scroll | 2,261 | 16.67 | 16.8 | 16.9 | 16.9 | 28% | 0.0% | 0.02 | 0.21 / 0.3 | 0.63 / 0.59 |
| 50,000 | webgpu | zoom | 14,449 | 16.67 | 16.8 | 16.8 | 16.8 | 35% | 0.0% | 0.01 | 0.18 / 0.3 | 0.70 / 0.92 |
| 50,000 | webgpu | drag | 8,913 | 16.67 | 16.8 | 16.8 | 16.8 | 37% | 0.0% | 0.01 | 0.10 / 0.2 | 0.60 / 0.59 |
| 50,000 | webgpu | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.8 | 16.9 | 50% | 0.0% | 6.89 | 0.32 / 0.4 | 0.57 / 0.59 |
| 50,000 | webgpu | playhead | 0 | 16.67 | 16.8 | 16.9 | 17.4 | 53% | 0.0% | 0.00 | 0.04 / 0.1 | n/a |
| 50,000 | webgpu | overview | 50,000 | 16.67 | 16.8 | 16.9 | 17.4 | 30% | 0.0% | 0.01 | 0.14 / 0.2 | 0.88 / 0.92 |

Three cells missed frames. `50,000 canvas2d overview` missed 5 of 234, with a worst frame of 50 ms. The other two are single frames (`10,000 webgl2 playhead`, one 32.1 ms frame, and `10,000 webgpu scroll`, one 26.1 ms frame) in scenarios that cost under 0.2 ms. A repeat of this whole matrix with 3 seconds per scenario missed no frames in 6,496, including `50,000 canvas2d overview`.

### WebView2, 1920×1080, device pixel ratio 1, 60 Hz

The same page in the real system web view.

| Notes | Renderer | Scenario | Notes in range | Frame avg | p95 | p99 | max | Over 16.7 ms | Missed | Update CPU | Draw CPU avg / p95 | GPU avg / p95 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 10,000 | canvas2d | scroll | 447 | 16.67 | 16.8 | 16.9 | 17.5 | 32% | 0.0% | 0.01 | 0.18 / 0.3 | n/a |
| 10,000 | canvas2d | zoom | 2,882 | 16.67 | 16.8 | 16.8 | 17.2 | 30% | 0.0% | 0.01 | 0.44 / 1.1 | n/a |
| 10,000 | canvas2d | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 38% | 0.0% | 0.01 | 0.33 / 0.4 | n/a |
| 10,000 | canvas2d | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.8 | 40% | 0.0% | 1.53 | 0.57 / 0.9 | n/a |
| 10,000 | canvas2d | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 48% | 0.0% | 0.00 | 0.05 / 0.1 | n/a |
| 10,000 | canvas2d | overview | 10,000 | 16.67 | 16.7 | 16.8 | 16.8 | 55% | 0.0% | 0.01 | 1.22 / 1.3 | n/a |
| 10,000 | webgl2 | scroll | 447 | 16.67 | 16.8 | 16.8 | 16.8 | 24% | 0.0% | 0.01 | 0.09 / 0.2 | 0.37 / 0.38 |
| 10,000 | webgl2 | zoom | 2,883 | 16.67 | 16.8 | 16.8 | 16.9 | 48% | 0.0% | 0.01 | 0.11 / 0.2 | 0.39 / 0.49 |
| 10,000 | webgl2 | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 57% | 0.0% | 0.00 | 0.05 / 0.1 | 0.43 / 0.43 |
| 10,000 | webgl2 | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.8 | 58% | 0.0% | 1.44 | 0.07 / 0.1 | 0.43 / 0.43 |
| 10,000 | webgl2 | playhead | 0 | 16.67 | 16.8 | 16.8 | 17.2 | 45% | 0.0% | 0.00 | 0.04 / 0.1 | n/a |
| 10,000 | webgl2 | overview | 10,000 | 16.67 | 16.8 | 16.8 | 16.9 | 30% | 0.0% | 0.01 | 0.10 / 0.2 | 0.57 / 0.53 |
| 10,000 | webgpu | scroll | 447 | 16.67 | 16.8 | 16.8 | 16.9 | 34% | 0.0% | 0.00 | 0.13 / 0.2 | 0.34 / 0.39 |
| 10,000 | webgpu | zoom | 2,883 | 16.67 | 16.8 | 16.8 | 16.8 | 37% | 0.0% | 0.01 | 0.17 / 0.3 | 0.46 / 1.64 |
| 10,000 | webgpu | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 41% | 0.0% | 0.01 | 0.07 / 0.2 | 0.41 / 0.39 |
| 10,000 | webgpu | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.9 | 45% | 0.0% | 1.44 | 0.13 / 0.2 | 0.57 / 2.10 |
| 10,000 | webgpu | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 47% | 0.0% | 0.00 | 0.06 / 0.1 | n/a |
| 10,000 | webgpu | overview | 10,000 | 16.67 | 16.8 | 16.9 | 17.4 | 32% | 0.0% | 0.01 | 0.16 / 0.3 | 0.66 / 2.23 |
| 50,000 | canvas2d | scroll | 2,261 | 16.67 | 16.8 | 16.8 | 16.8 | 26% | 0.0% | 0.01 | 0.42 / 0.7 | n/a |
| 50,000 | canvas2d | zoom | 14,596 | 16.67 | 16.8 | 16.8 | 16.8 | 28% | 0.0% | 0.01 | 2.22 / 6.8 | n/a |
| 50,000 | canvas2d | drag | 8,913 | 16.67 | 16.8 | 16.9 | 16.9 | 47% | 0.0% | 0.01 | 1.25 / 1.5 | n/a |
| 50,000 | canvas2d | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.8 | 16.9 | 50% | 0.0% | 6.89 | 2.19 / 2.3 | n/a |
| 50,000 | canvas2d | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 56% | 0.0% | 0.01 | 0.06 / 0.1 | n/a |
| 50,000 | canvas2d | overview | 50,000 | 17.09 | 16.8 | 33.4 | 33.5 | 57% | 2.6% | 0.01 | 8.11 / 8.7 | n/a |
| 50,000 | webgl2 | scroll | 2,261 | 16.67 | 16.8 | 17.0 | 17.1 | 32% | 0.0% | 0.01 | 0.12 / 0.2 | 0.72 / 2.23 |
| 50,000 | webgl2 | zoom | 14,448 | 16.67 | 16.8 | 16.8 | 16.8 | 41% | 0.0% | 0.01 | 0.13 / 0.2 | 1.04 / 3.61 |
| 50,000 | webgl2 | drag | 8,911 | 16.67 | 16.8 | 16.8 | 16.9 | 55% | 0.0% | 0.01 | 0.04 / 0.1 | 0.66 / 0.63 |
| 50,000 | webgl2 | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.8 | 16.8 | 57% | 0.0% | 6.87 | 0.10 / 0.2 | 0.64 / 0.62 |
| 50,000 | webgl2 | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 46% | 0.0% | 0.00 | 0.05 / 0.1 | n/a |
| 50,000 | webgl2 | overview | 50,000 | 16.67 | 16.8 | 16.8 | 17.2 | 31% | 0.0% | 0.01 | 0.12 / 0.2 | 1.63 / 4.88 |
| 50,000 | webgpu | scroll | 2,257 | 16.67 | 16.8 | 16.8 | 16.8 | 23% | 0.0% | 0.01 | 0.15 / 0.3 | 0.62 / 0.59 |
| 50,000 | webgpu | zoom | 14,596 | 16.67 | 16.8 | 16.8 | 17.6 | 37% | 0.0% | 0.01 | 0.16 / 0.3 | 0.95 / 2.75 |
| 50,000 | webgpu | drag | 8,913 | 16.67 | 16.8 | 16.9 | 17.3 | 40% | 0.0% | 0.01 | 0.08 / 0.2 | 0.67 / 2.03 |
| 50,000 | webgpu | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.8 | 17.2 | 43% | 0.0% | 7.52 | 0.35 / 0.4 | 0.55 / 0.59 |
| 50,000 | webgpu | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 46% | 0.0% | 0.00 | 0.06 / 0.2 | n/a |
| 50,000 | webgpu | overview | 50,000 | 16.67 | 16.8 | 16.8 | 16.8 | 30% | 0.0% | 0.01 | 0.14 / 0.3 | 0.92 / 0.85 |

The only missed frames in 8,647 were 6 in `50,000 canvas2d overview`. WebView2 and Edge are the same within noise, which is what sharing an engine and a version predicts.

### Edge, device pixel ratio 2 (emulated), 3840×2160 canvas, 60 Hz

| Notes | Renderer | Scenario | Notes in range | Frame avg | p95 | p99 | max | Over 16.7 ms | Missed | Update CPU | Draw CPU avg / p95 | GPU avg / p95 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 10,000 | canvas2d | scroll | 447 | 16.67 | 16.8 | 16.8 | 17.9 | 26% | 0.0% | 0.01 | 0.20 / 0.3 | n/a |
| 10,000 | canvas2d | zoom | 2,883 | 16.67 | 16.8 | 16.8 | 16.8 | 25% | 0.0% | 0.01 | 0.45 / 1.1 | n/a |
| 10,000 | canvas2d | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 44% | 0.0% | 0.01 | 0.40 / 0.6 | n/a |
| 10,000 | canvas2d | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.8 | 45% | 0.0% | 1.40 | 0.53 / 0.6 | n/a |
| 10,000 | canvas2d | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.9 | 46% | 0.0% | 0.01 | 0.07 / 0.1 | n/a |
| 10,000 | canvas2d | overview | 10,000 | 16.67 | 16.8 | 16.8 | 17.4 | 43% | 0.0% | 0.01 | 1.19 / 1.3 | n/a |
| 10,000 | webgl2 | scroll | 447 | 16.67 | 16.9 | 17.4 | 17.6 | 32% | 0.0% | 0.00 | 0.08 / 0.2 | 1.71 / 4.19 |
| 10,000 | webgl2 | zoom | 2,912 | 16.66 | 16.9 | 17.2 | 17.3 | 44% | 0.0% | 0.01 | 0.09 / 0.2 | 1.80 / 4.31 |
| 10,000 | webgl2 | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 55% | 0.0% | 0.01 | 0.05 / 0.1 | 0.45 / 1.51 |
| 10,000 | webgl2 | edit-rebuild | 1,640 | 16.67 | 16.8 | 17.3 | 17.5 | 57% | 0.0% | 1.41 | 0.05 / 0.1 | 1.83 / 3.05 |
| 10,000 | webgl2 | playhead | 0 | 16.67 | 16.8 | 17.1 | 17.2 | 45% | 0.0% | 0.00 | 0.06 / 0.1 | n/a |
| 10,000 | webgl2 | overview | 10,000 | 16.67 | 16.8 | 17.2 | 17.4 | 32% | 0.0% | 0.01 | 0.11 / 0.2 | 2.29 / 4.01 |
| 10,000 | webgpu | scroll | 447 | 16.67 | 16.8 | 16.8 | 16.9 | 23% | 0.0% | 0.01 | 0.16 / 0.3 | 3.19 / 4.85 |
| 10,000 | webgpu | zoom | 2,912 | 16.67 | 16.8 | 17.2 | 17.4 | 42% | 0.0% | 0.01 | 0.14 / 0.2 | 3.70 / 5.57 |
| 10,000 | webgpu | drag | 1,774 | 16.67 | 16.8 | 16.8 | 17.5 | 50% | 0.0% | 0.01 | 0.09 / 0.2 | 3.45 / 5.83 |
| 10,000 | webgpu | edit-rebuild | 1,640 | 16.67 | 16.8 | 16.8 | 16.9 | 51% | 0.0% | 1.55 | 0.15 / 0.2 | 0.12 / 0.26 |
| 10,000 | webgpu | playhead | 0 | 16.67 | 16.8 | 16.9 | 17.2 | 44% | 0.0% | 0.00 | 0.06 / 0.2 | n/a |
| 10,000 | webgpu | overview | 10,000 | 16.67 | 16.8 | 16.8 | 16.8 | 33% | 0.0% | 0.01 | 0.17 / 0.3 | 0.71 / 2.82 |
| 50,000 | canvas2d | scroll | 2,259 | 16.67 | 16.8 | 17.1 | 17.4 | 33% | 0.0% | 0.01 | 0.39 / 0.5 | n/a |
| 50,000 | canvas2d | zoom | 14,447 | 16.67 | 16.9 | 17.2 | 17.6 | 38% | 0.0% | 0.01 | 2.13 / 6.6 | n/a |
| 50,000 | canvas2d | drag | 8,913 | 16.67 | 16.8 | 16.9 | 17.1 | 38% | 0.0% | 0.01 | 1.20 / 1.3 | n/a |
| 50,000 | canvas2d | edit-rebuild | 8,245 | 16.67 | 16.7 | 16.8 | 16.8 | 49% | 0.0% | 6.90 | 2.18 / 2.3 | n/a |
| 50,000 | canvas2d | playhead | 0 | 16.67 | 16.8 | 17.0 | 18.1 | 45% | 0.0% | 0.00 | 0.06 / 0.1 | n/a |
| 50,000 | canvas2d | overview | 50,000 | 16.67 | 16.8 | 16.9 | 16.9 | 43% | 0.0% | 0.01 | 7.02 / 7.4 | n/a |
| 50,000 | webgl2 | scroll | 2,260 | 16.67 | 16.8 | 16.8 | 18.0 | 23% | 0.0% | 0.01 | 0.12 / 0.2 | 2.22 / 3.97 |
| 50,000 | webgl2 | zoom | 14,448 | 16.67 | 16.8 | 16.8 | 16.9 | 44% | 0.0% | 0.01 | 0.15 / 0.3 | 0.70 / 1.91 |
| 50,000 | webgl2 | drag | 8,913 | 16.67 | 16.8 | 16.8 | 16.9 | 51% | 0.0% | 0.01 | 0.05 / 0.1 | 4.34 / 6.44 |
| 50,000 | webgl2 | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.9 | 17.5 | 48% | 0.0% | 6.94 | 0.11 / 0.2 | 2.24 / 2.38 |
| 50,000 | webgl2 | playhead | 0 | 16.67 | 16.8 | 16.8 | 17.8 | 38% | 0.0% | 0.00 | 0.06 / 0.1 | n/a |
| 50,000 | webgl2 | overview | 50,000 | 16.67 | 16.9 | 17.3 | 18.7 | 37% | 0.0% | 0.01 | 0.11 / 0.2 | 3.14 / 5.73 |
| 50,000 | webgpu | scroll | 2,258 | 16.67 | 16.8 | 17.2 | 17.4 | 32% | 0.0% | 0.01 | 0.14 / 0.2 | 3.18 / 5.05 |
| 50,000 | webgpu | zoom | 14,448 | 16.67 | 16.8 | 17.0 | 17.1 | 38% | 0.0% | 0.01 | 0.15 / 0.2 | 2.86 / 5.96 |
| 50,000 | webgpu | drag | 8,913 | 16.67 | 16.9 | 17.2 | 17.7 | 45% | 0.0% | 0.01 | 0.07 / 0.2 | 3.48 / 5.44 |
| 50,000 | webgpu | edit-rebuild | 8,245 | 16.67 | 16.8 | 16.9 | 17.0 | 55% | 0.0% | 7.02 | 0.31 / 0.4 | 2.60 / 2.75 |
| 50,000 | webgpu | playhead | 0 | 16.67 | 16.9 | 17.0 | 17.4 | 50% | 0.0% | 0.00 | 0.06 / 0.1 | n/a |
| 50,000 | webgpu | overview | 50,000 | 16.67 | 16.9 | 17.2 | 17.3 | 28% | 0.0% | 0.01 | 0.15 / 0.3 | 3.30 / 6.42 |

No missed frames in 8,652. Four times the pixels raises GPU time to 2 to 4 ms in most cells at 60 Hz pacing. Under continuous load the same draws take 0.1 to 0.2 ms. That run is in the results file as `edge-uncapped-dpr2-emulated`.

### Edge, vsync off, 1920×1080, device pixel ratio 1

3 seconds per scenario. Frames per second here is throughput, not a display rate.

| Notes | Renderer | Scenario | Frames per second | Frame avg | p95 | p99 | max | Update CPU | Draw CPU | GPU avg / p95 |
|---|---|---|---|---|---|---|---|---|---|---|
| 10,000 | canvas2d | scroll | 3,713 | 0.27 | 0.4 | 0.4 | 2.2 | 0.00 | 0.12 | n/a |
| 10,000 | canvas2d | zoom | 2,256 | 0.44 | 1.7 | 3.6 | 18.0 | 0.00 | 0.17 | n/a |
| 10,000 | canvas2d | drag | 1,424 | 0.70 | 2.1 | 2.4 | 17.5 | 0.00 | 0.27 | n/a |
| 10,000 | canvas2d | edit-rebuild | 480 | 2.08 | 2.5 | 3.2 | 4.6 | 1.37 | 0.48 | n/a |
| 10,000 | canvas2d | playhead | 2,715 | 0.37 | 0.4 | 0.8 | 18.7 | 0.00 | 0.03 | n/a |
| 10,000 | canvas2d | overview | 308 | 3.24 | 6.0 | 6.7 | 7.3 | 0.00 | 1.19 | n/a |
| 10,000 | webgl2 | scroll | 2,861 | 0.35 | 0.4 | 1.1 | 18.5 | 0.00 | 0.04 | 0.03 / 0.03 |
| 10,000 | webgl2 | zoom | 3,291 | 0.30 | 0.3 | 0.6 | 18.7 | 0.00 | 0.05 | 0.03 / 0.04 |
| 10,000 | webgl2 | drag | 1,022 | 0.98 | 0.9 | 18.0 | 18.9 | 0.00 | 0.02 | 0.04 / 0.04 |
| 10,000 | webgl2 | edit-rebuild | 643 | 1.55 | 1.7 | 3.1 | 6.9 | 1.36 | 0.03 | 0.04 / 0.04 |
| 10,000 | webgl2 | playhead | 2,775 | 0.36 | 0.3 | 16.7 | 18.8 | 0.00 | 0.03 | n/a |
| 10,000 | webgl2 | overview | 4,031 | 0.25 | 0.4 | 0.5 | 18.7 | 0.00 | 0.06 | 0.04 / 0.05 |
| 10,000 | webgpu | scroll | 3,611 | 0.28 | 0.3 | 0.5 | 18.7 | 0.00 | 0.05 | 0.03 / 0.07 |
| 10,000 | webgpu | zoom | 3,733 | 0.27 | 0.3 | 0.4 | 18.4 | 0.00 | 0.06 | 0.03 / 0.07 |
| 10,000 | webgpu | drag | 1,744 | 0.57 | 0.4 | 17.5 | 18.7 | 0.00 | 0.04 | 0.05 / 0.07 |
| 10,000 | webgpu | edit-rebuild | 613 | 1.63 | 1.8 | 3.2 | 7.2 | 1.38 | 0.10 | 0.05 / 0.07 |
| 10,000 | webgpu | playhead | 2,779 | 0.36 | 0.4 | 0.9 | 19.1 | 0.00 | 0.04 | n/a |
| 10,000 | webgpu | overview | 4,035 | 0.25 | 0.3 | 0.4 | 19.0 | 0.00 | 0.07 | 0.04 / 0.07 |
| 50,000 | canvas2d | scroll | 1,150 | 0.87 | 1.8 | 2.5 | 3.3 | 0.00 | 0.33 | n/a |
| 50,000 | canvas2d | zoom | 1,113 | 0.90 | 3.2 | 11.3 | 32.0 | 0.00 | 0.51 | n/a |
| 50,000 | canvas2d | drag | 277 | 3.61 | 6.2 | 7.3 | 17.3 | 0.00 | 1.15 | n/a |
| 50,000 | canvas2d | edit-rebuild | 100 | 9.95 | 10.7 | 11.0 | 11.9 | 6.93 | 2.15 | n/a |
| 50,000 | canvas2d | playhead | 2,717 | 0.37 | 0.3 | 16.7 | 18.8 | 0.00 | 0.03 | n/a |
| 50,000 | canvas2d | overview | 64 | 15.68 | 32.7 | 33.9 | 34.3 | 0.01 | 14.82 | n/a |
| 50,000 | webgl2 | scroll | 2,829 | 0.35 | 0.4 | 1.5 | 19.0 | 0.00 | 0.04 | 0.05 / 0.05 |
| 50,000 | webgl2 | zoom | 3,261 | 0.31 | 0.3 | 0.5 | 18.5 | 0.00 | 0.05 | 0.06 / 0.12 |
| 50,000 | webgl2 | drag | 1,025 | 0.98 | 1.3 | 18.0 | 19.3 | 0.00 | 0.02 | 0.06 / 0.06 |
| 50,000 | webgl2 | edit-rebuild | 137 | 7.30 | 8.4 | 10.5 | 15.2 | 6.84 | 0.10 | 0.36 / 0.65 |
| 50,000 | webgl2 | playhead | 2,721 | 0.37 | 0.3 | 16.8 | 18.6 | 0.00 | 0.03 | n/a |
| 50,000 | webgl2 | overview | 3,640 | 0.27 | 0.4 | 0.6 | 18.3 | 0.00 | 0.07 | 0.12 / 0.15 |
| 50,000 | webgpu | scroll | 3,571 | 0.28 | 0.3 | 0.5 | 19.1 | 0.00 | 0.06 | 0.04 / 0.07 |
| 50,000 | webgpu | zoom | 3,477 | 0.29 | 0.3 | 0.5 | 18.7 | 0.00 | 0.06 | 0.05 / 0.07 |
| 50,000 | webgpu | drag | 1,708 | 0.59 | 0.4 | 17.4 | 18.8 | 0.00 | 0.03 | 0.04 / 0.07 |
| 50,000 | webgpu | edit-rebuild | 132 | 7.58 | 9.1 | 10.4 | 10.8 | 6.92 | 0.30 | 0.18 / 0.20 |
| 50,000 | webgpu | playhead | 2,727 | 0.37 | 0.3 | 16.9 | 18.8 | 0.00 | 0.03 | n/a |
| 50,000 | webgpu | overview | 3,985 | 0.25 | 0.3 | 0.4 | 18.6 | 0.00 | 0.08 | 0.06 / 0.07 |

WebGL2 and WebGPU are limited by the browser's own per-frame overhead of about 0.25 ms, not by the notes. 10,000 and 50,000 notes give nearly the same rate. Canvas 2D scales with the number of notes on screen. Its `overview` frame at 50,000 notes takes 15.7 ms, which is why it sits on the edge at 60 Hz.

For Canvas 2D, compare Draw CPU with the frame time. At 10,000 notes the draw call returns in 1.2 ms but the frame takes 3.2 ms. The other 2 ms is rasterization in the GPU process that main-thread timing never sees.

The same runs in WebView2 are in the results file as `webview2-uncapped-dpr1`. They match, with 4,109 fps for `10,000 webgl2 overview` and 60 fps for `50,000 canvas2d overview`.

### Past 50,000 notes

Where each renderer breaks. `scroll`, `edit-rebuild` and `overview` only, 3 seconds each.

| Notes | Renderer | Scenario | 60 Hz frame avg | p99 | max | Missed | Update CPU | Draw CPU | GPU avg | Uncapped frames per second |
|---|---|---|---|---|---|---|---|---|---|---|
| 100,000 | canvas2d | scroll | 16.67 | 16.8 | 16.8 | 0.0% | 0.01 | 0.67 | n/a | 493 |
| 100,000 | canvas2d | edit-rebuild | 19.35 | 33.4 | 33.5 | 16.1% | 14.01 | 4.99 | n/a | 52 |
| 100,000 | canvas2d | overview | 31.10 | 50.1 | 50.1 | 72.2% | 0.01 | 28.84 | n/a | 32 |
| 100,000 | webgl2 | scroll | 16.67 | 17.3 | 17.9 | 0.0% | 0.00 | 0.09 | 0.83 | 2,824 |
| 100,000 | webgl2 | edit-rebuild | 16.67 | 17.2 | 17.6 | 0.0% | 13.77 | 0.28 | 0.97 | 71 |
| 100,000 | webgl2 | overview | 16.67 | 16.9 | 17.0 | 0.0% | 0.01 | 0.11 | 2.06 | 3,523 |
| 100,000 | webgpu | scroll | 16.67 | 18.4 | 19.3 | 0.0% | 0.01 | 0.14 | 0.85 | 3,502 |
| 100,000 | webgpu | edit-rebuild | 16.67 | 17.1 | 18.3 | 0.0% | 14.09 | 0.70 | 0.76 | 66 |
| 100,000 | webgpu | overview | 16.67 | 16.8 | 16.9 | 0.0% | 0.01 | 0.13 | 1.39 | 4,064 |
| 200,000 | canvas2d | scroll | 16.67 | 16.8 | 16.8 | 0.0% | 0.01 | 1.20 | n/a | 285 |
| 200,000 | canvas2d | edit-rebuild | 39.18 | 50.1 | 50.1 | 100.0% | 28.52 | 9.92 | n/a | 26 |
| 200,000 | canvas2d | overview | 63.50 | 83.4 | 83.4 | 100.0% | 0.01 | 62.12 | n/a | 16 |
| 200,000 | webgl2 | scroll | 16.67 | 17.6 | 17.7 | 0.0% | 0.00 | 0.12 | 0.79 | 2,535 |
| 200,000 | webgl2 | edit-rebuild | 29.13 | 33.5 | 33.7 | 74.8% | 28.54 | 0.34 | 1.73 | 34 |
| 200,000 | webgl2 | overview | 16.67 | 16.9 | 17.5 | 0.0% | 0.01 | 0.18 | 3.69 | 2,140 |
| 200,000 | webgpu | scroll | 16.67 | 17.6 | 18.4 | 0.0% | 0.01 | 0.15 | 2.36 | 3,700 |
| 200,000 | webgpu | edit-rebuild | 30.47 | 49.9 | 49.9 | 81.8% | 29.06 | 1.13 | 1.28 | 33 |
| 200,000 | webgpu | overview | 16.67 | 17.2 | 17.8 | 0.0% | 0.01 | 0.17 | 4.17 | 4,142 |
| 500,000 | canvas2d | scroll | 16.67 | 16.9 | 16.9 | 0.0% | 0.01 | 3.70 | n/a | 113 |
| 500,000 | canvas2d | edit-rebuild | 104.61 | 133.3 | 133.3 | 100.0% | 78.05 | 26.67 | n/a | 10 |
| 500,000 | canvas2d | overview | 183.33 | 233.4 | 233.4 | 100.0% | 0.01 | 181.32 | n/a | 5 |
| 500,000 | webgl2 | scroll | 16.67 | 16.9 | 20.4 | 0.0% | 0.01 | 0.14 | 3.42 | 2,601 |
| 500,000 | webgl2 | edit-rebuild | 79.39 | 116.6 | 116.6 | 100.0% | 77.87 | 1.13 | 5.06 | 13 |
| 500,000 | webgl2 | overview | 16.67 | 16.9 | 17.3 | 0.0% | 0.01 | 0.12 | 3.65 | 736 |
| 500,000 | webgpu | scroll | 16.67 | 17.3 | 17.5 | 0.0% | 0.01 | 0.16 | 2.86 | 3,766 |
| 500,000 | webgpu | edit-rebuild | 80.70 | 116.6 | 116.6 | 100.0% | 78.45 | 1.39 | 2.50 | 12 |
| 500,000 | webgpu | overview | 16.67 | 16.9 | 17.2 | 0.0% | 0.01 | 0.16 | 4.70 | 2,409 |
| 1,000,000 | canvas2d | scroll | 17.65 | 33.4 | 33.4 | 5.9% | 0.01 | 12.09 | n/a | 57 |
| 1,000,000 | canvas2d | edit-rebuild | 250.00 | 266.8 | 266.8 | 100.0% | 155.70 | 94.77 | n/a | 4 |
| 1,000,000 | canvas2d | overview | 372.23 | 450.1 | 450.1 | 100.0% | 0.00 | 412.85 | n/a | 3 |
| 1,000,000 | webgl2 | scroll | 16.67 | 17.2 | 17.3 | 0.0% | 0.01 | 0.10 | 4.20 | 2,353 |
| 1,000,000 | webgl2 | edit-rebuild | 161.41 | 183.3 | 183.3 | 100.0% | 154.42 | 6.34 | 7.17 | 6 |
| 1,000,000 | webgl2 | overview | 16.67 | 17.2 | 17.7 | 0.0% | 0.01 | 0.13 | 5.27 | 312 |
| 1,000,000 | webgpu | scroll | 16.67 | 16.8 | 17.2 | 0.0% | 0.00 | 0.15 | 3.21 | 2,089 |
| 1,000,000 | webgpu | edit-rebuild | 159.65 | 166.8 | 166.8 | 100.0% | 155.21 | 2.62 | 2.84 | 6 |
| 1,000,000 | webgpu | overview | 16.67 | 16.9 | 17.2 | 0.0% | 0.00 | 0.17 | 5.03 | 1,304 |

- WebGL2 and WebGPU draw 1,000,000 notes on screen at 60 fps. GPU time is about 5 ms at 60 Hz pacing. With vsync off WebGL2 manages 312 fps and WebGPU 1,304 fps, the one place WebGPU is clearly faster.
- Canvas 2D breaks between 50,000 and 100,000 notes on screen. In normal use only the visible notes are drawn, so scrolling a 500,000-note pattern at working zoom still holds 60 fps.
- `edit-rebuild` breaks first, and for every renderer at the same place, because the cost is JavaScript and not drawing. Rebuilding the batch costs 1.4 ms at 10,000 notes, 7 ms at 50,000, 14 ms at 100,000 and 28 ms at 200,000. Above about 100,000 notes in one pattern, an edit cannot rebuild everything inside one frame.

### GPU disabled

Edge started with `--disable-gpu`. `edge://gpu` reported everything "Software only" and WebGL ran on SwiftShader. 3 seconds per scenario, no GPU timing.

| Notes | Renderer | Scenario | Notes in range | Frame avg | p95 | p99 | max | Over 16.7 ms | Missed | Update CPU | Draw CPU avg / p95 | GPU avg / p95 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 10,000 | canvas2d | scroll | 447 | 16.67 | 16.8 | 16.8 | 16.8 | 27% | 0.0% | 0.01 | 0.20 / 0.3 | n/a |
| 10,000 | canvas2d | zoom | 2,883 | 16.67 | 16.7 | 16.8 | 16.8 | 13% | 0.0% | 0.01 | 0.47 / 1.2 | n/a |
| 10,000 | canvas2d | drag | 1,774 | 16.67 | 16.8 | 16.8 | 16.8 | 36% | 0.0% | 0.01 | 0.31 / 0.4 | n/a |
| 10,000 | canvas2d | edit-rebuild | 1,640 | 16.67 | 16.7 | 16.8 | 16.8 | 36% | 0.0% | 1.42 | 0.55 / 0.7 | n/a |
| 10,000 | canvas2d | playhead | 0 | 16.67 | 16.7 | 16.8 | 16.8 | 39% | 0.0% | 0.01 | 0.08 / 0.2 | n/a |
| 10,000 | canvas2d | overview | 10,000 | 16.67 | 16.7 | 16.8 | 16.8 | 46% | 0.0% | 0.02 | 1.31 / 2.1 | n/a |
| 10,000 | webgl2 | scroll | 446 | 16.67 | 16.7 | 16.8 | 16.8 | 32% | 0.0% | 0.01 | 0.17 / 0.3 | n/a |
| 10,000 | webgl2 | zoom | 1,489 | 22.85 | 66.6 | 83.3 | 83.4 | 30% | 18.9% | 0.02 | 0.16 / 0.2 | n/a |
| 10,000 | webgl2 | drag | 1,773 | 29.41 | 33.4 | 33.4 | 33.4 | 89% | 76.5% | 0.01 | 0.08 / 0.2 | n/a |
| 10,000 | webgl2 | edit-rebuild | 1,640 | 30.17 | 33.4 | 33.4 | 50.0 | 95% | 80.0% | 1.72 | 0.09 / 0.2 | n/a |
| 10,000 | webgl2 | playhead | 0 | 16.67 | 16.7 | 16.8 | 16.8 | 59% | 0.0% | 0.00 | 0.08 / 0.2 | n/a |
| 10,000 | webgl2 | overview | 10,000 | 84.73 | 100.0 | 100.0 | 100.0 | 100% | 100.0% | 0.02 | 0.19 / 0.3 | n/a |
| 10,000 | webgpu | all | failed to start: webgpu renderer unavailable: no adapter | | | | | | | | | |
| 50,000 | canvas2d | scroll | 2,260 | 16.67 | 16.7 | 16.8 | 16.8 | 32% | 0.0% | 0.01 | 0.46 / 0.8 | n/a |
| 50,000 | canvas2d | zoom | 11,924 | 18.51 | 33.3 | 49.9 | 50.0 | 33% | 9.8% | 0.01 | 5.31 / 19.2 | n/a |
| 50,000 | canvas2d | drag | 8,911 | 16.67 | 16.7 | 16.8 | 16.8 | 34% | 0.0% | 0.01 | 1.45 / 2.2 | n/a |
| 50,000 | canvas2d | edit-rebuild | 8,248 | 21.43 | 33.4 | 33.4 | 33.5 | 57% | 28.6% | 9.26 | 2.89 / 3.8 | n/a |
| 50,000 | canvas2d | playhead | 0 | 16.67 | 16.7 | 16.8 | 16.8 | 44% | 0.0% | 0.01 | 0.08 / 0.2 | n/a |
| 50,000 | canvas2d | overview | 50,000 | 26.84 | 33.4 | 33.4 | 50.0 | 81% | 60.2% | 0.01 | 24.47 / 26.3 | n/a |
| 50,000 | webgl2 | scroll | 2,258 | 25.86 | 33.4 | 33.4 | 33.4 | 61% | 55.2% | 0.01 | 0.18 / 0.3 | n/a |
| 50,000 | webgl2 | zoom | 3,869 | 38.62 | 116.6 | 366.8 | 366.8 | 39% | 32.9% | 0.01 | 0.16 / 0.3 | n/a |
| 50,000 | webgl2 | drag | 8,895 | 119.23 | 133.4 | 133.4 | 133.4 | 100% | 100.0% | 0.00 | 0.05 / 0.1 | n/a |
| 50,000 | webgl2 | edit-rebuild | 8,245 | 117.95 | 133.4 | 133.4 | 133.4 | 100% | 100.0% | 8.88 | 0.24 / 0.4 | n/a |
| 50,000 | webgl2 | playhead | 0 | 16.67 | 16.8 | 16.8 | 16.8 | 39% | 0.0% | 0.00 | 0.09 / 0.2 | n/a |
| 50,000 | webgl2 | overview | 50,000 | 375.00 | 383.4 | 383.4 | 383.4 | 100% | 100.0% | 0.03 | 0.24 / 0.7 | n/a |
| 50,000 | webgpu | all | failed to start: webgpu renderer unavailable: no adapter | | | | | | | | | |

Canvas 2D on the CPU holds 60 fps at 10,000 notes in every scenario. At 50,000 it drops to 37 fps with every note on screen and misses frames in `zoom` and `edit-rebuild`. WebGL2 on the software rasterizer runs at 12 fps with 10,000 notes on screen and 3 fps with 50,000. Its Draw CPU column still reads 0.2 ms while the frame takes 85 ms, which is the reason this document never judges a renderer by main-thread time alone. This CPU has 32 threads, so a typical machine will do worse in both columns.

A current Edge or WebView2 without Playwright's `--enable-unsafe-swiftshader` flag may refuse to create a software WebGL context at all. Either way `createRenderer("auto")` ends up on Canvas 2D. It asks for WebGL2 with `failIfMajorPerformanceCaveat` and also checks the reported GPU name. I ran `renderer=auto` with the GPU on and off, and it picked WebGL2 and Canvas 2D.

### The three renderers draw the same picture

Screenshots of the same scene (10,000 notes, a selection being dragged, dark and light themes) were compared pixel by pixel over a 1300×1080 region.

- WebGL2 against WebGPU: 0 differing pixels in both themes.
- WebGL2 against Canvas 2D: about 2% of pixels differ. All of them are where two notes overlap on the same row (Canvas 2D groups its fills by color, so it paints overlapping notes in a different order) or are a one-step rounding difference on some note borders.

## Recommendation

**Use WebGL2 for the piano roll and the playlist, with Canvas 2D as the automatic fallback.** That is `createRenderer("auto")`, the default of `TimeGridView`.

Measured headroom for WebGL2 on this machine, with every note on screen:

| Notes | Main thread per frame | GPU per frame at 60 Hz | Share of the 16.7 ms budget | Rate with vsync off |
|---|---|---|---|---|
| 10,000 | 0.09 ms | 0.59 ms | about 4% | 4,031 fps |
| 50,000 | 0.14 ms | 0.94 ms | about 6% | 3,640 fps |
| 1,000,000 | 0.13 ms | 5.27 ms | about 32% | 312 fps |

The cost does not grow with the note count in any way that matters, because nothing per note happens on the main thread. Notes are uploaded once. Scrolling, zooming and dragging change a handful of uniforms. The playhead and marquee are on a separate overlay canvas, so playback redraws one rectangle.

Why not the others:

- **WebGPU** is not worth carrying as the default. Same speed, identical output, asynchronous start-up, and no recovery from a lost device in this implementation. Its one win, 1,000,000 notes with vsync off, is outside anything a DAW does. The renderer exists and passes the benchmark, so it can stay as an option or be deleted. It should not be in the automatic order until a platform needs it.
- **Canvas 2D** is good enough to ship as the fallback and not good enough to be the default. It has about 5 times headroom at 10,000 notes on this machine, which could be gone on a laptop with integrated graphics at 4K. It is the right choice with no GPU.

Three rules for whoever builds the piano roll on this, each backed by a number above:

1. **Drag with `setDragOffset`, commit on drop.** Dragging 1,000 notes this way costs 0.05 ms per frame at both 10,000 and 50,000 notes. Dispatching a command per frame and rebuilding costs 7 ms per frame at 50,000 notes.
2. **Rebuild the batch once per edit, not once per frame.** One rebuild is 1.4 ms at 10,000 notes, which is fine for an edit. If patterns above 100,000 notes ever matter, the batch needs in-place updates, which `RectBatch` does not have yet.
3. **Feed the playhead through `setPlayhead`.** It touches the overlay only. Do not invalidate the base layer from the realtime feed.

## What this says about falling back to Electron

On Windows the system web view is not the limit, and Electron would not draw faster. WebView2 154 and Edge 154 are the same Chromium engine, which is also what Electron bundles, and the WebView2 control measured the same as Edge here. At these sizes the limit is not the web view at all. The first thing to break is JavaScript rebuilding a batch for a 200,000-note pattern.

This says nothing yet about macOS and Linux. WKWebView and WebKitGTK are different engines with different canvas and WebGL implementations, and neither was measured. The plan already calls WebKitGTK the weak one. The Electron decision for Linux should wait for a run of this same benchmark there. It takes a few minutes. Start the dev server, open `bench.html?auto=1&renderer=webgl2&notes=10000` in the Tauri window and read `window.__benchResults`.

Two things already reduce the risk on those platforms. The renderer falls back to Canvas 2D on its own if WebGL2 is missing or software-only. And Canvas 2D has enough headroom at 10,000 notes here that a web view several times slower would still pass.

## Not measured

- macOS WKWebView and Linux WebKitGTK. Nothing in this document applies to them.
- The Tauri shell itself. WebView2 was measured in a WinForms host with the same runtime and version. Tauri's window options were not.
- Any other hardware. One desktop with a fast discrete GPU. Integrated graphics, laptops on battery and older GPUs were not measured. The WebGL2 workload is small (one draw call, no textures, at most a few million pixels of fill), so I expect it to hold, but that is an expectation.
- A real high-DPI display. Device pixel ratio 2 was emulated, and the display was never driven at 4K.
- Displays faster than 60 Hz. The measured costs fit a 144 Hz frame (6.9 ms) on paper. Not tested.
- Text inside notes, and a real ruler and keyboard. The overlay drew about 20 text labels per frame and nothing more.
- The rest of the app. The page had one canvas and one small DOM readout. The real window will have the shell, meters redrawing at 60 Hz, the realtime feed and other panels sharing the main thread.
- Several canvases at once. Chromium caps the live WebGL contexts in a page, at 16 on desktop as far as I know. I did not test the cap. One context per editor panel is fine. A context per meter or per waveform thumbnail is not, and those should use Canvas 2D.
- GPU time for Canvas 2D, for the reason given under Method.
- WebGL context loss. The renderer handles `webglcontextlost` and `webglcontextrestored`, but no test forces them.
- Memory use and long sessions.
- Input latency. Frames were counted. The time from a mouse move to the pixels changing was not measured.

## Reproducing

```sh
# terminal 1, from apps/desktop
pnpm exec vite --port 1433 --strictPort

# terminal 2, from any scratch folder with playwright-core installed
node <repo>/docs/perf/run-canvas-bench.mjs --out results.json
node <repo>/docs/perf/run-canvas-bench.mjs --dpr 2 --out results-dpr2.json
node <repo>/docs/perf/run-canvas-bench.mjs --uncapped --seconds 3 --out results-uncapped.json
node <repo>/docs/perf/run-canvas-bench.mjs --browser-args="--disable-gpu" --out results-software.json
```

For WebView2, start `docs/perf/webview2-host.ps1` (its header says how) and add `--cdp http://127.0.0.1:9333`.

To try it by hand, open `http://localhost:1433/bench.html?notes=50000&renderer=webgl2&theme=dark`. Wheel scrolls, Ctrl+wheel zooms time, Alt+wheel zooms rows, dragging empty space selects, dragging a note moves the selection, Space runs a playhead. The parameters are `notes`, `renderer` (`auto`, `canvas2d`, `webgl2`, `webgpu`), `theme` (`dark`, `light`), `auto=1`, `seconds`, `gpuSeconds`, `scenarios` and `labels=0`.

## What the piano roll builds on

`apps/desktop/src/lib/canvas` has no dependencies and does not import React, the store or IPC. Import from `@/lib/canvas`. The React wrapper is `@/lib/canvas/TimeGridCanvas`.

| Piece | What it is |
|---|---|
| `TimeGridView` | Owns two stacked canvases in a container: a base canvas (grid and items, drawn by the renderer) and an overlay canvas (marquee, playhead, custom painters). Handles resize, device pixel ratio and theme changes. `create(container, options)`, `setViewport`, `panBy`, `zoomTime`, `zoomRows`, `setItems`, `setDragOffset`, `setMarquee`, `setPlayhead`, `addOverlayPainter`, `onThemeChange`, `hitTest`, `invalidate`, `flush`, `destroy`. |
| `Viewport` and its functions | Plain data plus pure math: `tickToX`, `xToTick`, `rowToY`, `yToRow`, `visibleTicks`, `visibleRows`, `scrollByPx`, `zoomTimeAt`, `zoomRowsAt`, `clampViewport`, `deviceTransform`, `deviceX`, `deviceY`, `snapTick`, `keyToRow`, `rowToKey`, `isBlackKey`. |
| `RectBatch` | Rectangles in tick and row space as typed arrays: start, length, row, row span, color, flags, id. Notes, clips and grid lines are all rect batches. Version counters tell a renderer what to upload. |
| `indexBatch`, `visibleRange`, `queryPoint`, `queryRect` | Sorts a batch by start and builds a time-bucket index. Culling and hit queries read a few buckets whatever the batch size. |
| `hitTestPoint`, `hitTestRect` | The same queries in CSS pixels. A point hit reports `body`, `start-edge` or `end-edge` for resize handles. |
| `buildNoteBatch`, `velocityPalette` | Turns `Note`-shaped objects into a batch, velocity to brightness. |
| `writeGrid`, `gridLevels`, `pianoRows`, `plainRows` | The background grid for one frame, with line levels that thin out as you zoom out. |
| `readGridTheme`, `observeTheme` | Reads `--background`, `--foreground`, `--muted-foreground`, `--wf-brand`, `--wf-playhead`, `--wf-grid-line` and `--wf-grid-line-strong` from computed style and reports changes. |
| `RectRenderer`, `createRenderer` | The renderer interface and its Canvas 2D, WebGL2 and WebGPU implementations. |

Limits worth knowing before building on it:

- One item batch per view. Ghost notes from other channels would need a second batch, which the view does not take yet.
- Tick attributes are 32-bit integers and the scroll position is subtracted before any float math, so long projects stay pixel exact.
- Notes that overlap on one row are painted in start order by the GPU renderers and in color order by Canvas 2D.
- Item colors are baked into the batch from the theme. Rebuild the batch in `onThemeChange`.
