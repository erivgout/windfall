import type { RectBatch } from "./rect-batch"
import {
  BORDER_SHADE,
  RendererUnavailableError,
  isSoftwareGpu,
  type DrawOptions,
  type RectRenderer,
  type RendererInfo,
} from "./renderer"
import type { GridTheme } from "./theme"
import type { DeviceTransform } from "./viewport"

// One instance per rect; the four corners come from gl_VertexID. Ticks stay
// integers until the scroll position has been subtracted, so a long project
// does not lose precision in float32.
const VERTEX_SHADER = `#version 300 es
precision highp float;
precision highp int;

layout(location = 0) in ivec4 a_geometry;
layout(location = 1) in vec4 a_color;
layout(location = 2) in uint a_flags;

uniform vec2 u_resolution;
uniform vec4 u_transform;
uniform ivec3 u_ticks;
uniform float u_lineWidth;
uniform int u_pass;
uniform vec4 u_selectionFill;
uniform vec3 u_selectionBorder;

flat out vec4 v_fill;
flat out vec4 v_border;
flat out vec2 v_size;
flat out float v_borderWidth;
out vec2 v_local;

float snap(float v) {
  return floor(v + 0.5);
}

void main() {
  bool selected = (a_flags & 1u) != 0u;
  if ((u_pass == 1 && selected) || (u_pass == 2 && !selected)) {
    gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
    return;
  }
  bool isFlat = (a_flags & 2u) != 0u;
  int start = a_geometry.x - u_ticks.x;
  int row = a_geometry.z;
  if (selected) {
    start += u_ticks.y;
    row += u_ticks.z;
  }
  float lw = u_lineWidth;
  float x0 = snap(float(start) * u_transform.x - u_transform.y);
  float x1 = snap(float(start + a_geometry.y) * u_transform.x - u_transform.y);
  float y0 = snap(float(row) * u_transform.z - u_transform.w);
  float y1 = snap(float(row + a_geometry.w) * u_transform.z - u_transform.w);
  if (!isFlat) y0 += lw;
  if ((a_flags & 4u) != 0u) { x0 = 0.0; x1 = u_resolution.x; }
  if ((a_flags & 8u) != 0u) { y0 = 0.0; y1 = u_resolution.y; }
  if ((a_flags & 16u) != 0u) x1 = x0 + lw;
  if ((a_flags & 32u) != 0u) y1 = y0 + lw;
  x1 = max(x1, x0 + lw);
  y1 = max(y1, y0 + lw);

  vec2 size = vec2(x1 - x0, y1 - y0);
  vec2 corner = vec2(float(gl_VertexID & 1), float(gl_VertexID >> 1));
  vec2 position = vec2(x0, y0) + corner * size;

  vec3 fill = a_color.rgb;
  vec3 border = fill * ${BORDER_SHADE};
  if (selected) {
    fill = mix(fill, u_selectionFill.rgb, u_selectionFill.a);
    border = u_selectionBorder;
  }
  v_fill = vec4(fill * a_color.a, a_color.a);
  v_border = vec4(border * a_color.a, a_color.a);
  v_size = size;
  v_borderWidth = (isFlat || size.x < 3.0 * lw || size.y < 3.0 * lw) ? 0.0 : lw;
  v_local = corner * size;
  gl_Position = vec4(
    position / u_resolution * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}
`

const FRAGMENT_SHADER = `#version 300 es
precision highp float;

flat in vec4 v_fill;
flat in vec4 v_border;
flat in vec2 v_size;
flat in float v_borderWidth;
in vec2 v_local;

out vec4 outColor;

void main() {
  vec2 edge = min(v_local, v_size - v_local);
  outColor = min(edge.x, edge.y) < v_borderWidth ? v_border : v_fill;
}
`

interface TimerQueryExtension {
  readonly TIME_ELAPSED_EXT: number
  readonly GPU_DISJOINT_EXT: number
}

interface DebugRendererInfo {
  readonly UNMASKED_RENDERER_WEBGL: number
}

interface GpuBatch {
  vao: WebGLVertexArrayObject
  geometry: WebGLBuffer
  colors: WebGLBuffer
  flags: WebGLBuffer
  /** Instances the buffers have room for. */
  capacity: number
  geometryVersion: number
  colorVersion: number
  flagsVersion: number
}

interface Uniforms {
  resolution: WebGLUniformLocation | null
  transform: WebGLUniformLocation | null
  ticks: WebGLUniformLocation | null
  lineWidth: WebGLUniformLocation | null
  pass: WebGLUniformLocation | null
  selectionFill: WebGLUniformLocation | null
  selectionBorder: WebGLUniformLocation | null
}

const PASS_ALL = 0
const PASS_UNSELECTED = 1
const PASS_SELECTED = 2

function compile(
  gl: WebGL2RenderingContext,
  type: number,
  source: string
): WebGLShader {
  const shader = gl.createShader(type)
  if (!shader) throw new RendererUnavailableError("webgl2", "no shader")
  gl.shaderSource(shader, source)
  gl.compileShader(shader)
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader) ?? "compile failed"
    gl.deleteShader(shader)
    throw new RendererUnavailableError("webgl2", log)
  }
  return shader
}

class WebGL2Renderer implements RectRenderer {
  readonly info: RendererInfo
  onRestored: (() => void) | null = null

  private readonly canvas: HTMLCanvasElement
  private readonly gl: WebGL2RenderingContext
  private readonly timerExtension: TimerQueryExtension | null
  private program: WebGLProgram | null = null
  private uniforms: Uniforms | null = null
  private batches = new Map<RectBatch, GpuBatch>()
  private theme: GridTheme | null = null
  private transform: DeviceTransform | null = null
  private timing = false
  private activeQuery: WebGLQuery | null = null
  private pendingQueries: WebGLQuery[] = []
  private lost = false

  constructor(canvas: HTMLCanvasElement, rejectSoftware: boolean) {
    const gl = canvas.getContext("webgl2", {
      // Makes the browser refuse a context that would run on the CPU.
      failIfMajorPerformanceCaveat: rejectSoftware,
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      powerPreference: "high-performance",
      preserveDrawingBuffer: false,
    })
    if (!gl) throw new RendererUnavailableError("webgl2", "no webgl2 context")
    this.canvas = canvas
    this.gl = gl
    this.timerExtension = gl.getExtension("EXT_disjoint_timer_query_webgl2")
    const debugInfo: DebugRendererInfo | null = gl.getExtension(
      "WEBGL_debug_renderer_info"
    )
    const device: unknown = gl.getParameter(
      debugInfo ? debugInfo.UNMASKED_RENDERER_WEBGL : gl.RENDERER
    )
    this.info = {
      kind: "webgl2",
      device: typeof device === "string" ? device : "unknown",
      gpuTiming: this.timerExtension ? "timer-query" : "none",
    }
    this.initialize()
    canvas.addEventListener("webglcontextlost", this.handleLost)
    canvas.addEventListener("webglcontextrestored", this.handleRestored)
  }

  resize(widthDev: number, heightDev: number): void {
    if (this.canvas.width !== widthDev) this.canvas.width = widthDev
    if (this.canvas.height !== heightDev) this.canvas.height = heightDev
  }

  setTheme(theme: GridTheme): void {
    this.theme = theme
  }

  beginFrame(transform: DeviceTransform): void {
    this.transform = transform
    const gl = this.gl
    const uniforms = this.uniforms
    if (this.lost || !uniforms) return
    if (this.timing && this.timerExtension && !this.activeQuery) {
      const query = gl.createQuery()
      if (query) {
        gl.beginQuery(this.timerExtension.TIME_ELAPSED_EXT, query)
        this.activeQuery = query
      }
    }
    const width = this.canvas.width
    const height = this.canvas.height
    gl.viewport(0, 0, width, height)
    const background = this.theme?.background
    if (background) {
      gl.clearColor(
        background.r / 255,
        background.g / 255,
        background.b / 255,
        1
      )
    }
    gl.clear(gl.COLOR_BUFFER_BIT)
    gl.useProgram(this.program)
    gl.uniform2f(uniforms.resolution, width, height)
    gl.uniform4f(
      uniforms.transform,
      transform.scaleX,
      transform.offsetX,
      transform.scaleY,
      transform.offsetY
    )
    gl.uniform1f(uniforms.lineWidth, transform.lineWidth)
    const theme = this.theme
    if (theme) {
      const fill = theme.selectionFill
      gl.uniform4f(
        uniforms.selectionFill,
        fill.r / 255,
        fill.g / 255,
        fill.b / 255,
        theme.selectionMix
      )
      const border = theme.selectionBorder
      gl.uniform3f(
        uniforms.selectionBorder,
        border.r / 255,
        border.g / 255,
        border.b / 255
      )
    }
  }

  drawBatch(batch: RectBatch, options: DrawOptions = {}): void {
    const gl = this.gl
    const uniforms = this.uniforms
    const transform = this.transform
    if (this.lost || !uniforms || !transform) return
    const first = Math.max(0, options.first ?? 0)
    const last = Math.min(batch.count, options.last ?? batch.count)
    if (last <= first) return

    const gpu = this.upload(batch)
    gl.bindVertexArray(gpu.vao)
    // WebGL2 has no base instance, so a sub-range is drawn by pointing the
    // attributes at its first rect.
    gl.bindBuffer(gl.ARRAY_BUFFER, gpu.geometry)
    gl.vertexAttribIPointer(0, 4, gl.INT, 16, first * 16)
    gl.bindBuffer(gl.ARRAY_BUFFER, gpu.colors)
    gl.vertexAttribPointer(1, 4, gl.UNSIGNED_BYTE, true, 4, first * 4)
    gl.bindBuffer(gl.ARRAY_BUFFER, gpu.flags)
    gl.vertexAttribIPointer(2, 1, gl.UNSIGNED_INT, 4, first * 4)

    const instances = last - first
    if (batch.selectedCount === 0) {
      gl.uniform3i(uniforms.ticks, transform.scrollTick, 0, 0)
      gl.uniform1i(uniforms.pass, PASS_ALL)
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, instances)
    } else {
      gl.uniform3i(
        uniforms.ticks,
        transform.scrollTick,
        options.dragTicks ?? 0,
        options.dragRows ?? 0
      )
      gl.uniform1i(uniforms.pass, PASS_UNSELECTED)
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, instances)
      gl.uniform1i(uniforms.pass, PASS_SELECTED)
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, instances)
    }
    gl.bindVertexArray(null)
  }

  endFrame(): void {
    this.transform = null
    if (this.activeQuery && this.timerExtension) {
      this.gl.endQuery(this.timerExtension.TIME_ELAPSED_EXT)
      this.pendingQueries.push(this.activeQuery)
      this.activeQuery = null
    }
  }

  finish(): boolean {
    if (this.lost) return false
    this.gl.finish()
    return true
  }

  release(batch: RectBatch): void {
    const gpu = this.batches.get(batch)
    if (!gpu) return
    this.destroyGpuBatch(gpu)
    this.batches.delete(batch)
  }

  setGpuTiming(enabled: boolean): boolean {
    this.timing = enabled && this.timerExtension !== null
    return this.timing
  }

  takeGpuTimes(): number[] {
    const gl = this.gl
    const extension = this.timerExtension
    const times: number[] = []
    if (!extension || this.lost) return times
    const disjoint = gl.getParameter(extension.GPU_DISJOINT_EXT) === true
    const waiting: WebGLQuery[] = []
    for (const query of this.pendingQueries) {
      if (gl.getQueryParameter(query, gl.QUERY_RESULT_AVAILABLE) !== true) {
        waiting.push(query)
        continue
      }
      const nanoseconds: unknown = gl.getQueryParameter(query, gl.QUERY_RESULT)
      // A disjoint event (a GPU reset, a power state change) makes the
      // elapsed time meaningless.
      if (!disjoint && typeof nanoseconds === "number") {
        times.push(nanoseconds / 1e6)
      }
      gl.deleteQuery(query)
    }
    this.pendingQueries = waiting
    return times
  }

  dispose(): void {
    this.canvas.removeEventListener("webglcontextlost", this.handleLost)
    this.canvas.removeEventListener("webglcontextrestored", this.handleRestored)
    const gl = this.gl
    for (const gpu of this.batches.values()) this.destroyGpuBatch(gpu)
    this.batches.clear()
    for (const query of this.pendingQueries) gl.deleteQuery(query)
    this.pendingQueries = []
    gl.deleteProgram(this.program)
    this.program = null
    // Browsers cap live WebGL contexts per page, so give this one back now
    // instead of waiting for garbage collection.
    gl.getExtension("WEBGL_lose_context")?.loseContext()
  }

  private initialize(): void {
    const gl = this.gl
    const program = gl.createProgram()
    if (!program) throw new RendererUnavailableError("webgl2", "no program")
    const vertex = compile(gl, gl.VERTEX_SHADER, VERTEX_SHADER)
    const fragment = compile(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER)
    gl.attachShader(program, vertex)
    gl.attachShader(program, fragment)
    gl.linkProgram(program)
    gl.deleteShader(vertex)
    gl.deleteShader(fragment)
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      const log = gl.getProgramInfoLog(program) ?? "link failed"
      gl.deleteProgram(program)
      throw new RendererUnavailableError("webgl2", log)
    }
    this.program = program
    this.uniforms = {
      resolution: gl.getUniformLocation(program, "u_resolution"),
      transform: gl.getUniformLocation(program, "u_transform"),
      ticks: gl.getUniformLocation(program, "u_ticks"),
      lineWidth: gl.getUniformLocation(program, "u_lineWidth"),
      pass: gl.getUniformLocation(program, "u_pass"),
      selectionFill: gl.getUniformLocation(program, "u_selectionFill"),
      selectionBorder: gl.getUniformLocation(program, "u_selectionBorder"),
    }
    gl.disable(gl.DEPTH_TEST)
    gl.enable(gl.BLEND)
    // Colors leave the vertex shader premultiplied.
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA)
  }

  private upload(batch: RectBatch): GpuBatch {
    const gl = this.gl
    let gpu = this.batches.get(batch)
    if (!gpu) {
      gpu = this.createGpuBatch()
      this.batches.set(batch, gpu)
    }
    const count = batch.count
    const grow = count > gpu.capacity
    if (grow) gpu.capacity = Math.max(count, gpu.capacity * 2, 64)

    if (grow || gpu.geometryVersion !== batch.geometryVersion) {
      gl.bindBuffer(gl.ARRAY_BUFFER, gpu.geometry)
      if (grow)
        gl.bufferData(gl.ARRAY_BUFFER, gpu.capacity * 16, gl.DYNAMIC_DRAW)
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, batch.geometry, 0, count * 4)
      gpu.geometryVersion = batch.geometryVersion
    }
    if (grow || gpu.colorVersion !== batch.colorVersion) {
      gl.bindBuffer(gl.ARRAY_BUFFER, gpu.colors)
      if (grow)
        gl.bufferData(gl.ARRAY_BUFFER, gpu.capacity * 4, gl.DYNAMIC_DRAW)
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, batch.colors, 0, count * 4)
      gpu.colorVersion = batch.colorVersion
    }
    if (grow || gpu.flagsVersion !== batch.flagsVersion) {
      gl.bindBuffer(gl.ARRAY_BUFFER, gpu.flags)
      if (grow)
        gl.bufferData(gl.ARRAY_BUFFER, gpu.capacity * 4, gl.DYNAMIC_DRAW)
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, batch.flags, 0, count)
      gpu.flagsVersion = batch.flagsVersion
    }
    return gpu
  }

  private createGpuBatch(): GpuBatch {
    const gl = this.gl
    const vao = gl.createVertexArray()
    const geometry = gl.createBuffer()
    const colors = gl.createBuffer()
    const flags = gl.createBuffer()
    if (!vao || !geometry || !colors || !flags) {
      throw new RendererUnavailableError("webgl2", "out of GPU memory")
    }
    gl.bindVertexArray(vao)
    for (let location = 0; location < 3; location++) {
      gl.enableVertexAttribArray(location)
      gl.vertexAttribDivisor(location, 1)
    }
    gl.bindVertexArray(null)
    return {
      vao,
      geometry,
      colors,
      flags,
      capacity: 0,
      geometryVersion: -1,
      colorVersion: -1,
      flagsVersion: -1,
    }
  }

  private destroyGpuBatch(gpu: GpuBatch): void {
    const gl = this.gl
    gl.deleteVertexArray(gpu.vao)
    gl.deleteBuffer(gpu.geometry)
    gl.deleteBuffer(gpu.colors)
    gl.deleteBuffer(gpu.flags)
  }

  private readonly handleLost = (event: Event): void => {
    // Without preventDefault the browser never restores the context.
    event.preventDefault()
    this.lost = true
    this.batches.clear()
    this.pendingQueries = []
    this.activeQuery = null
  }

  private readonly handleRestored = (): void => {
    this.lost = false
    this.initialize()
    this.onRestored?.()
  }
}

export interface WebGL2Options {
  /** Fail instead of running on a software rasterizer. Defaults to false. */
  readonly rejectSoftware?: boolean
}

export function createWebGL2Renderer(
  canvas: HTMLCanvasElement,
  options: WebGL2Options = {}
): RectRenderer {
  const rejectSoftware = options.rejectSoftware ?? false
  const renderer = new WebGL2Renderer(canvas, rejectSoftware)
  // Not every browser honors failIfMajorPerformanceCaveat, so check the
  // name it reports as well.
  if (rejectSoftware && isSoftwareGpu(renderer.info.device)) {
    renderer.dispose()
    throw new RendererUnavailableError(
      "webgl2",
      `software rasterizer (${renderer.info.device})`
    )
  }
  return renderer
}
