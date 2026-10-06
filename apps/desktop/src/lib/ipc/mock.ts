import type {
  AudioHost,
  AudioSettings,
  BrowserRoot,
  Command,
  DispatchResult,
  DocumentSnapshot,
  EngineStatus,
  ExportProgress,
  Project,
  ProjectPatch,
  RealtimeFrame,
  TransportState,
} from "@/bindings"

import type { Backend, Unsubscribe } from "./backend"
import {
  baseName,
  defaultRoots,
  FACTORY_ROOT,
  isAudioPath,
  listFolder,
  sampleInfoFor,
  samplePathFor,
} from "./sim/browser"
import { SimDocument } from "./sim/document"
import { demoProject, newProject } from "./sim/project"
import { emptyTouched, type Touched } from "./sim/touched"
import { TransportSim } from "./sim/transport"

/** Stand-ins for the native file dialogs. Each resolves to null on cancel. */
export type MockDialogs = {
  openProject(saved: string[]): Promise<string | null>
  saveProject(suggestedPath: string): Promise<string | null>
  exportPath(suggestedPath: string): Promise<string | null>
  folder(): Promise<string | null>
}

export type MockOptions = {
  /** Where "saved" projects live. Pass null to keep them in memory only. */
  storage?: Pick<Storage, "getItem" | "setItem"> | null
  dialogs?: MockDialogs
  /** The project to start with. Defaults to the demo beat. */
  project?: Project
}

const FILES_KEY = "windfall.mock.files"
const RECENT_KEY = "windfall.mock.recent"
const FRAME_MS = 1000 / 60

const HOSTS: AudioHost[] = [
  {
    name: "WASAPI",
    isDefault: true,
    devices: [
      {
        name: "Speakers (simulated)",
        isDefault: true,
        sampleRates: [44_100, 48_000, 96_000],
        minBufferFrames: 64,
        maxBufferFrames: 4096,
      },
      {
        name: "Headphones (simulated)",
        isDefault: false,
        sampleRates: [44_100, 48_000],
        minBufferFrames: 128,
        maxBufferFrames: 2048,
      },
    ],
  },
  {
    name: "ASIO",
    isDefault: false,
    devices: [
      {
        name: "Audio interface (simulated)",
        isDefault: true,
        sampleRates: [44_100, 48_000, 88_200, 96_000, 192_000],
        minBufferFrames: 32,
        maxBufferFrames: 2048,
      },
    ],
  },
]

function promptDialogs(): MockDialogs {
  const ask = (message: string, suggestion: string) =>
    Promise.resolve(window.prompt(message, suggestion)?.trim() || null)
  return {
    openProject(saved) {
      if (saved.length === 0) {
        return Promise.reject(
          new Error("No projects are saved in this browser yet.")
        )
      }
      return ask(
        `Open project. Saved in this browser:\n${saved.join("\n")}`,
        saved[0]
      )
    },
    saveProject: (suggested) => ask("Save project as", suggested),
    exportPath: (suggested) => ask("Export audio to", suggested),
    folder: () => ask("Folder to add", "/samples/My samples"),
  }
}

class Emitter<T> {
  private handlers = new Set<(value: T) => void>()

  on(handler: (value: T) => void): Unsubscribe {
    this.handlers.add(handler)
    return () => {
      this.handlers.delete(handler)
    }
  }

  emit(value: T) {
    for (const handler of [...this.handlers]) handler(value)
  }

  get size() {
    return this.handlers.size
  }
}

function describeStatus(settings: AudioSettings): EngineStatus {
  const host =
    HOSTS.find((item) => item.name === settings.host) ??
    HOSTS.find((item) => item.isDefault) ??
    HOSTS[0]
  const device =
    settings.device === undefined
      ? host.devices.find((item) => item.isDefault)
      : host.devices.find((item) => item.name === settings.device)
  const sampleRate = settings.sampleRate ?? 48_000
  const bufferFrames = settings.bufferFrames ?? 256
  const base = {
    host: host.name,
    sampleRate,
    bufferFrames,
    latencyMs: (bufferFrames / sampleRate) * 1000,
  }
  if (settings.host !== undefined && host.name !== settings.host) {
    return {
      ...base,
      running: false,
      device: null,
      error: `The audio driver "${settings.host}" is not available.`,
    }
  }
  if (!device) {
    return {
      ...base,
      running: false,
      device: null,
      error: `The device "${settings.device}" was not found.`,
    }
  }
  if (!device.sampleRates.includes(sampleRate)) {
    return {
      ...base,
      running: false,
      device: device.name,
      error: `${device.name} does not support ${sampleRate} Hz.`,
    }
  }
  return { ...base, running: true, device: device.name, error: null }
}

/**
 * The whole backend, in the browser. It keeps a project, applies every
 * command the way the Rust document does, and fakes the engine: a playhead
 * that follows the tempo and meters that move with the beat.
 */
export function createMockBackend(options: MockOptions = {}): Backend {
  const storage =
    options.storage === undefined
      ? typeof localStorage === "undefined"
        ? null
        : localStorage
      : options.storage
  const dialogs = options.dialogs ?? promptDialogs()

  let doc = new SimDocument(options.project ?? demoProject())
  let path: string | null = null
  let transport = new TransportSim(doc.project().patterns[0].id)
  let engine = describeStatus({})
  let roots: BrowserRoot[] = defaultRoots()
  let memoryFiles: Record<string, Project> = {}
  let memoryRecent: string[] = []

  const patches = new Emitter<ProjectPatch>()
  const loaded = new Emitter<DocumentSnapshot>()
  const transportStates = new Emitter<TransportState>()
  const engineStatuses = new Emitter<EngineStatus>()
  const exportProgress = new Emitter<ExportProgress>()
  const frames = new Emitter<RealtimeFrame>()

  let frameTimer: ReturnType<typeof setInterval> | null = null
  let lastFrameAt = 0

  function readJson<T>(key: string, fallback: T): T {
    try {
      const text = storage?.getItem(key)
      return text ? (JSON.parse(text) as T) : fallback
    } catch {
      return fallback
    }
  }

  function files(): Record<string, Project> {
    return storage ? readJson(FILES_KEY, {}) : memoryFiles
  }

  function recent(): string[] {
    return storage ? readJson(RECENT_KEY, []) : memoryRecent
  }

  function remember(savedPath: string, project: Project) {
    const nextFiles = { ...files(), [savedPath]: project }
    const nextRecent = [
      savedPath,
      ...recent().filter((item) => item !== savedPath),
    ].slice(0, 8)
    if (!storage) {
      memoryFiles = nextFiles
      memoryRecent = nextRecent
      return
    }
    try {
      storage.setItem(FILES_KEY, JSON.stringify(nextFiles))
      storage.setItem(RECENT_KEY, JSON.stringify(nextRecent))
    } catch {
      throw new Error(
        "The browser's storage is full, so the project was not saved."
      )
    }
  }

  function emitTransport(): TransportState {
    transportStates.emit(transport.state)
    return transport.state
  }

  /** The transport's pattern can vanish on a delete or an undo. */
  function repairTransportPattern() {
    const patterns = doc.project().patterns
    if (patterns.some((pattern) => pattern.id === transport.state.pattern)) {
      return
    }
    transport.set({ pattern: patterns[0].id })
    emitTransport()
  }

  function publish(touched: Touched): ProjectPatch {
    const patch = doc.patch(touched)
    repairTransportPattern()
    patches.emit(patch)
    return patch
  }

  function dispatchNow(command: Command, gesture?: number): DispatchResult {
    const applied = doc.dispatch(command, gesture)
    return { created: applied.created, patch: publish(applied.touched) }
  }

  function load(project: Project, loadedPath: string | null) {
    doc = new SimDocument(project)
    path = loadedPath
    transport = new TransportSim(project.patterns[0].id)
    const snapshot = doc.snapshot(path)
    loaded.emit(snapshot)
    emitTransport()
    return snapshot
  }

  function startFrames() {
    if (frameTimer !== null) return
    lastFrameAt = performance.now()
    frameTimer = setInterval(() => {
      const now = performance.now()
      const seconds = Math.min(0.25, (now - lastFrameAt) / 1000)
      lastFrameAt = now
      const wasPlaying = transport.state.playing
      frames.emit(transport.advance(doc.project(), seconds))
      // Song mode stops by itself at the end of the last clip.
      if (wasPlaying && !transport.state.playing) emitTransport()
    }, FRAME_MS)
  }

  function stopFrames() {
    if (frameTimer === null || frames.size > 0) return
    clearInterval(frameTimer)
    frameTimer = null
  }

  /** Runs a call the way IPC does: later, and failing with a plain message. */
  function ipc<T>(work: () => T): Promise<T> {
    return new Promise((resolve, reject) => {
      queueMicrotask(() => {
        try {
          resolve(work())
        } catch (error) {
          reject(error instanceof Error ? error : new Error(String(error)))
        }
      })
    })
  }

  function sampleCommands(filePath: string) {
    if (!isAudioPath(filePath)) {
      throw new Error(`"${filePath}" is not an audio file Windfall can read`)
    }
    const samplePath = samplePathFor(filePath)
    const project = doc.project()
    const existing = project.samples.find(
      (sample) =>
        sample.path.kind === samplePath.kind &&
        sample.path.path === samplePath.path
    )
    const name = baseName(filePath)
    return {
      name,
      // Ids are handed out in order, so the id a new sample will get is known.
      sampleId: existing?.id ?? project.nextId,
      addSample: { type: "addSample", name, path: samplePath } as const,
    }
  }

  return {
    kind: "mock",

    documentSnapshot: () => ipc(() => doc.snapshot(path)),
    dispatch: (command, gesture) => ipc(() => dispatchNow(command, gesture)),
    undo: () =>
      ipc(() => {
        const touched = doc.undo()
        return touched ? publish(touched) : null
      }),
    redo: () =>
      ipc(() => {
        const touched = doc.redo()
        return touched ? publish(touched) : null
      }),
    historyJump: (cursor) => ipc(() => publish(doc.jump(cursor))),

    projectNew: () => ipc(() => load(newProject(), null)),
    projectOpen: (openPath) =>
      ipc(() => {
        const project = files()[openPath]
        if (!project) throw new Error(`Could not open "${openPath}".`)
        remember(openPath, project)
        return load(project, openPath)
      }),
    projectSave: (savePath) =>
      ipc(() => {
        const target = savePath ?? path
        if (!target) {
          throw new Error("This project has no file yet. Use Save as.")
        }
        remember(target, doc.project())
        path = target
        doc.markSaved()
        publish(emptyTouched())
        return target
      }),
    recentProjects: () => ipc(() => recent()),

    transportPlay: () =>
      ipc(() => {
        transport.play()
        return emitTransport()
      }),
    transportStop: () =>
      ipc(() => {
        transport.stop()
        return emitTransport()
      }),
    transportToggle: () =>
      ipc(() => {
        if (transport.state.playing) transport.stop()
        else transport.play()
        return emitTransport()
      }),
    transportSeek: (tick) => ipc(() => transport.seek(tick)),
    transportSet: (patch) =>
      ipc(() => {
        if (
          patch.pattern !== undefined &&
          !doc
            .project()
            .patterns.some((pattern) => pattern.id === patch.pattern)
        ) {
          throw new Error(`pattern ${patch.pattern} does not exist`)
        }
        transport.set(patch)
        return emitTransport()
      }),
    transportState: () => ipc(() => transport.state),

    engineStatus: () => ipc(() => engine),
    engineDevices: () => ipc(() => HOSTS),
    engineConfigure: (settings) =>
      ipc(() => {
        engine = describeStatus(settings)
        engineStatuses.emit(engine)
        return engine
      }),

    auditionNoteOn: () => ipc(() => undefined),
    auditionNoteOff: () => ipc(() => undefined),
    previewPlay: (filePath) =>
      ipc(() => {
        sampleInfoFor(roots, filePath)
      }),
    previewStop: () => ipc(() => undefined),

    browserRoots: () => ipc(() => roots),
    browserAddRoot: (rootPath) =>
      ipc(() => {
        const clean = rootPath.replace(/[\\/]+$/, "")
        if (!clean) throw new Error("Choose a folder to add.")
        if (roots.some((root) => root.path === clean)) {
          throw new Error(`"${clean}" is already in the browser.`)
        }
        const name = clean.split(/[\\/]/).pop() ?? clean
        roots = [...roots, { name, path: clean, kind: "user" }]
        return roots
      }),
    browserRemoveRoot: (rootPath) =>
      ipc(() => {
        const root = roots.find((item) => item.path === rootPath)
        if (!root) throw new Error(`"${rootPath}" is not in the browser.`)
        if (root.kind === "factory") {
          throw new Error("The factory library cannot be removed.")
        }
        roots = roots.filter((item) => item.path !== rootPath)
        return roots
      }),
    browserList: (folderPath) => ipc(() => listFolder(roots, folderPath)),
    sampleInfo: (filePath) => ipc(() => sampleInfoFor(roots, filePath)),
    sampleInfoById: (sample) =>
      ipc(() => {
        const asset = doc.project().samples.find((item) => item.id === sample)
        if (!asset) throw new Error(`sample ${sample} does not exist`)
        const filePath =
          asset.path.kind === "factory"
            ? `${FACTORY_ROOT}/${asset.path.path}`
            : asset.path.path
        return sampleInfoFor(roots, filePath)
      }),

    addChannelFromFile: (filePath, index) =>
      ipc(() => {
        const { name, sampleId, addSample } = sampleCommands(filePath)
        return dispatchNow({
          type: "batch",
          label: "Add channel",
          commands: [
            addSample,
            { type: "addChannel", name, sample: sampleId, index },
          ],
        })
      }),
    setChannelSampleFromFile: (channel, filePath) =>
      ipc(() => {
        const { sampleId, addSample } = sampleCommands(filePath)
        return dispatchNow({
          type: "batch",
          label: "Change channel sample",
          commands: [
            addSample,
            { type: "setChannelSample", id: channel, sample: sampleId },
          ],
        })
      }),

    exportAudio: (exportOptions) =>
      ipc(() => {
        if (!exportOptions.path)
          throw new Error("Choose where to save the file.")
        if (exportOptions.patternLoops < 1) {
          throw new Error("Render the pattern at least once.")
        }
        let fraction = 0
        const timer = setInterval(() => {
          fraction = Math.min(1, fraction + 0.06)
          const done = fraction >= 1
          exportProgress.emit({
            path: exportOptions.path,
            fraction,
            done,
            error: null,
          })
          if (done) clearInterval(timer)
        }, 60)
      }),

    onProjectPatch: (handler) => patches.on(handler),
    onProjectLoaded: (handler) => loaded.on(handler),
    onTransportState: (handler) => transportStates.on(handler),
    onEngineStatus: (handler) => engineStatuses.on(handler),
    onExportProgress: (handler) => exportProgress.on(handler),
    // The mock never loads a project with missing files.
    onProjectWarnings: () => () => undefined,

    subscribeRealtime(handler) {
      const off = frames.on(handler)
      startFrames()
      return () => {
        off()
        stopFrames()
      }
    },

    pickProjectToOpen: () => dialogs.openProject(recent()),
    pickProjectSavePath: (suggestedName) =>
      dialogs.saveProject(`/projects/${suggestedName}.windfall`),
    pickExportPath: (suggestedName) =>
      dialogs.exportPath(`/exports/${suggestedName}.wav`),
    pickFolder: () => dialogs.folder(),

    setWindowTitle: (title) =>
      ipc(() => {
        if (typeof window !== "undefined") window.document.title = title
      }),

    // A browser tab cannot wait for an answer before it closes, so the mock
    // falls back to the browser's own "leave site?" prompt.
    onCloseRequested() {
      const handler = (event: BeforeUnloadEvent) => {
        if (doc.isDirty()) event.preventDefault()
      }
      window.addEventListener("beforeunload", handler)
      return () => window.removeEventListener("beforeunload", handler)
    },
  }
}
