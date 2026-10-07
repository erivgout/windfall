import { Channel, invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { open, save } from "@tauri-apps/plugin-dialog"

import type { RealtimeFrame } from "@/bindings"

import {
  AUDIO_EXTENSIONS,
  EVENTS,
  errorMessage,
  type Backend,
  type Unsubscribe,
} from "./backend"

/** Rejections from Rust are plain strings; the UI wants `Error`s. */
async function call<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (error) {
    throw new Error(errorMessage(error), { cause: error })
  }
}

function on<T>(event: string, handler: (payload: T) => void): Unsubscribe {
  let cancelled = false
  let unlisten: Unsubscribe | null = null
  void listen<T>(event, (message) => handler(message.payload)).then((fn) => {
    if (cancelled) fn()
    else unlisten = fn
  })
  return () => {
    cancelled = true
    unlisten?.()
  }
}

const PROJECT_FILTER = [{ name: "Windfall project", extensions: ["windfall"] }]

export function createTauriBackend(): Backend {
  const realtimeHandlers = new Set<(frame: RealtimeFrame) => void>()
  let realtimeStarted = false

  // The shell streams frames for the life of the window, so one channel is
  // opened on first use and shared by every subscriber.
  function startRealtime() {
    if (realtimeStarted) return
    realtimeStarted = true
    const channel = new Channel<RealtimeFrame>()
    channel.onmessage = (frame) => {
      for (const handler of realtimeHandlers) handler(frame)
    }
    call<void>("realtime_subscribe", { channel }).catch((error: unknown) => {
      realtimeStarted = false
      console.error("realtime_subscribe failed:", errorMessage(error))
    })
  }

  return {
    kind: "tauri",

    documentSnapshot: () => call("document_snapshot"),
    dispatch: (command, gesture) => call("dispatch", { command, gesture }),
    undo: () => call("undo"),
    redo: () => call("redo"),
    historyJump: (cursor) => call("history_jump", { cursor }),

    projectNew: () => call("project_new"),
    projectOpen: (path) => call("project_open", { path }),
    projectSave: (path) => call("project_save", { path }),
    recentProjects: () => call("recent_projects"),

    transportPlay: () => call("transport_play"),
    transportStop: () => call("transport_stop"),
    transportToggle: () => call("transport_toggle"),
    transportSeek: (tick) => call("transport_seek", { tick }),
    transportSet: (patch) => call("transport_set", { patch }),
    transportState: () => call("transport_state"),

    engineStatus: () => call("engine_status"),
    engineDevices: () => call("engine_devices"),
    engineConfigure: (settings) => call("engine_configure", { settings }),
    engineSettings: () => call("engine_settings"),

    auditionNoteOn: (channel, key, velocity) =>
      call("audition_note_on", { channel, key, velocity }),
    auditionNoteOff: (channel, key) =>
      call("audition_note_off", { channel, key }),
    previewPlay: (path) => call("preview_play", { path }),
    previewStop: () => call("preview_stop"),

    browserRoots: () => call("browser_roots"),
    browserAddRoot: (path) => call("browser_add_root", { path }),
    browserRemoveRoot: (path) => call("browser_remove_root", { path }),
    browserList: (path) => call("browser_list", { path }),
    sampleInfo: (path) => call("sample_info", { path }),
    prepareClipCommand: (command) => call("prepare_clip_command", { command }),
    detectClipTempo: (sample) => call("detect_clip_tempo", { sample }),
    sampleInfoById: (sample) => call("sample_info_by_id", { sample }),
    addChannelFromFile: (path, index) =>
      call("add_channel_from_file", { path, index }),
    setChannelSampleFromFile: (channel, path) =>
      call("set_channel_sample_from_file", { channel, path }),
    addAudioClipFromFile: (path, place) =>
      call("add_audio_clip_from_file", { path, ...place }),
    addAudioClipFromSample: (sample, place) =>
      call("add_audio_clip_from_sample", { sample, ...place }),
    automate: (target) => call("automate", { target }),
    samplesReload: () => call("samples_reload"),

    exportAudio: (options) => call("export_audio", { options }),
    exportCancel: () => call("export_cancel"),

    onProjectPatch: (handler) => on(EVENTS.projectPatch, handler),
    onProjectLoaded: (handler) => on(EVENTS.projectLoaded, handler),
    onTransportState: (handler) => on(EVENTS.transportState, handler),
    onEngineStatus: (handler) => on(EVENTS.engineStatus, handler),
    onExportProgress: (handler) => on(EVENTS.exportProgress, handler),
    onProjectWarnings: (handler) => on(EVENTS.projectWarnings, handler),

    subscribeRealtime(handler) {
      realtimeHandlers.add(handler)
      startRealtime()
      return () => {
        realtimeHandlers.delete(handler)
      }
    },

    async pickProjectToOpen() {
      const picked = await open({
        title: "Open project",
        multiple: false,
        directory: false,
        filters: PROJECT_FILTER,
      })
      return typeof picked === "string" ? picked : null
    },
    pickProjectSavePath: (suggestedName) =>
      save({
        title: "Save project as",
        defaultPath: `${suggestedName}.windfall`,
        filters: PROJECT_FILTER,
      }),
    pickExportPath: (suggestedName, format = "wav") =>
      save({
        title: "Export audio",
        defaultPath: `${suggestedName}.${format}`,
        filters: [
          { name: `${format.toUpperCase()} audio`, extensions: [format] },
        ],
      }),
    async pickFolder() {
      const picked = await open({
        title: "Choose a folder",
        multiple: false,
        directory: true,
      })
      return typeof picked === "string" ? picked : null
    },

    async pickAudioFile() {
      const picked = await open({
        title: "Choose an audio file",
        multiple: false,
        directory: false,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      })
      return typeof picked === "string" ? picked : null
    },

    setWindowTitle: (title) => getCurrentWindow().setTitle(title),

    onCloseRequested(guard) {
      const window = getCurrentWindow()
      let cancelled = false
      let unlisten: Unsubscribe | null = null
      void window
        .onCloseRequested(async (event) => {
          // The default close has to be stopped synchronously; the window is
          // destroyed by hand once the guard agrees.
          event.preventDefault()
          if (!(await guard())) return
          try {
            await window.destroy()
          } catch (error) {
            // Needs core:window:allow-destroy in the shell's capabilities.
            console.error("The window could not close:", errorMessage(error))
          }
        })
        .then((fn) => {
          if (cancelled) fn()
          else unlisten = fn
        })
      return () => {
        cancelled = true
        unlisten?.()
      }
    },
  }
}
