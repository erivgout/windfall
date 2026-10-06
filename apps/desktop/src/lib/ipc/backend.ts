import type {
  AudioHost,
  AudioSettings,
  BrowserEntry,
  BrowserRoot,
  ChannelId,
  Command,
  DispatchResult,
  DocumentSnapshot,
  EngineStatus,
  ExportOptions,
  ExportProgress,
  ProjectPatch,
  RealtimeFrame,
  SampleId,
  SampleInfo,
  TransportPatch,
  TransportState,
} from "@/bindings"

export type Unsubscribe = () => void

/**
 * Everything the UI can ask of the app shell. One method per IPC command in
 * docs/ARCHITECTURE.md, plus events, file dialogs and the few window calls
 * the shell needs. A failed call rejects with an `Error` whose message is
 * plain text that can be shown to the user.
 */
export interface Backend {
  /** "tauri" inside the app, "mock" in a plain browser. */
  readonly kind: "tauri" | "mock"

  documentSnapshot(): Promise<DocumentSnapshot>
  dispatch(command: Command, gesture?: number): Promise<DispatchResult>
  undo(): Promise<ProjectPatch | null>
  redo(): Promise<ProjectPatch | null>
  historyJump(cursor: number): Promise<ProjectPatch>

  projectNew(): Promise<DocumentSnapshot>
  projectOpen(path: string): Promise<DocumentSnapshot>
  /** Resolves to the saved path. Rejects when there is no path yet. */
  projectSave(path?: string): Promise<string>
  recentProjects(): Promise<string[]>

  transportPlay(): Promise<TransportState>
  transportStop(): Promise<TransportState>
  transportToggle(): Promise<TransportState>
  transportSeek(tick: number): Promise<void>
  transportSet(patch: TransportPatch): Promise<TransportState>
  transportState(): Promise<TransportState>

  engineStatus(): Promise<EngineStatus>
  engineDevices(): Promise<AudioHost[]>
  engineConfigure(settings: AudioSettings): Promise<EngineStatus>

  auditionNoteOn(
    channel: ChannelId,
    key: number,
    velocity: number
  ): Promise<void>
  auditionNoteOff(channel: ChannelId, key: number): Promise<void>
  previewPlay(path: string): Promise<void>
  previewStop(): Promise<void>

  browserRoots(): Promise<BrowserRoot[]>
  browserAddRoot(path: string): Promise<BrowserRoot[]>
  browserRemoveRoot(path: string): Promise<BrowserRoot[]>
  /** Folders first, then by name. */
  browserList(path: string): Promise<BrowserEntry[]>
  sampleInfo(path: string): Promise<SampleInfo>
  /** Facts and waveform of a sample in the project's pool. */
  sampleInfoById(sample: SampleId): Promise<SampleInfo>
  /** One undo step: adds the sample to the pool and a channel that plays it. */
  addChannelFromFile(path: string, index?: number): Promise<DispatchResult>
  setChannelSampleFromFile(
    channel: ChannelId,
    path: string
  ): Promise<DispatchResult>

  /** Returns at once. Progress arrives through `onExportProgress`. */
  exportAudio(options: ExportOptions): Promise<void>

  onProjectPatch(handler: (patch: ProjectPatch) => void): Unsubscribe
  onProjectLoaded(handler: (snapshot: DocumentSnapshot) => void): Unsubscribe
  onTransportState(handler: (state: TransportState) => void): Unsubscribe
  onEngineStatus(handler: (status: EngineStatus) => void): Unsubscribe
  onExportProgress(handler: (progress: ExportProgress) => void): Unsubscribe
  /** Problems found while loading a project, such as a missing sample. */
  onProjectWarnings(handler: (warnings: string[]) => void): Unsubscribe
  /** Frames arrive about 60 times a second while subscribed. */
  subscribeRealtime(handler: (frame: RealtimeFrame) => void): Unsubscribe

  /** File dialogs. Each resolves to `null` when the user cancels. */
  pickProjectToOpen(): Promise<string | null>
  pickProjectSavePath(suggestedName: string): Promise<string | null>
  pickExportPath(suggestedName: string): Promise<string | null>
  pickFolder(): Promise<string | null>

  setWindowTitle(title: string): Promise<void>
  /**
   * Asks `guard` before the window closes. The window closes only when it
   * resolves to true.
   */
  onCloseRequested(guard: () => Promise<boolean>): Unsubscribe
}

export const EVENTS = {
  projectPatch: "project:patch",
  projectLoaded: "project:loaded",
  transportState: "transport:state",
  engineStatus: "engine:status",
  exportProgress: "export:progress",
  projectWarnings: "project:warnings",
} as const

/** Turns whatever a backend call rejected with into a readable message. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error
  if (error instanceof Error) return error.message
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message
  }
  return "Something went wrong."
}
