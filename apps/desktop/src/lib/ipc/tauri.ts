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
const MIDI_FILTER = [{ name: "MIDI file", extensions: ["mid", "midi"] }]

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
    timelineState: () => call("timeline_state"),
    timelineRegion: (region, generation, revision, request) =>
      call("timeline_region", { region, generation, revision, request }),
    midiHardwareState: () => call("midi_hardware_state"),
    midiHardwareRefresh: () => call("midi_hardware_refresh"),
    midiHardwareConfigure: (settings) =>
      call("midi_hardware_configure", { settings }),
    midiHardwareTarget: (channel, generation, revision) =>
      call("midi_hardware_target", { channel, generation, revision }),
    midiHardwarePanic: () => call("midi_hardware_panic"),
    audioEditorOpen: (clip) => call("audio_editor_open", { clip }),
    audioEditorApply: (request) => call("audio_editor_apply", { request }),
    audioEditorDiscard: (token) => call("audio_editor_discard", { token }),
    sliceAnalyze: (clip, options) => call("slice_analyze", { clip, options }),
    sliceApply: (token, markers) => call("slice_apply", { token, markers }),
    sliceDiscard: (token) => call("slice_discard", { token }),
    pluginsState: () => call("plugins_state"),
    pluginsScan: (retry) => call("plugins_scan", { retry }),
    pluginsAddFolder: (folder) => call("plugins_add_folder", { folder }),
    pluginsAdd: (path, id, track) => call("plugins_add", { path, id, track }),
    pluginEditor: (target, open) => call("plugin_editor", { target, open }),

    documentSnapshot: () => call("document_snapshot"),
    dispatch: (command, gesture) => call("dispatch", { command, gesture }),
    undo: () => call("undo"),
    redo: () => call("redo"),
    historyJump: (cursor) => call("history_jump", { cursor }),

    flpPreview: (path, options) => call("flp_preview", { path, options }),
    flpOpen: (token) => call("flp_open", { token }),
    flpCancel: (token) => call("flp_cancel", { token }),
    async pickFlpFile() {
      const picked = await open({
        title: "Import FL Studio project",
        multiple: false,
        directory: false,
        filters: [{ name: "FL Studio project", extensions: ["flp"] }],
      })
      return typeof picked === "string" ? picked : null
    },

    projectNew: () => call("project_new"),
    projectOpen: (path) => call("project_open", { path }),
    projectSave: (path) => call("project_save", { path }),
    projectSaveNewVersion: (path) => call("project_save_new_version", { path }),
    projectArchiveSave: (path) => call("project_archive_save", { path }),
    projectArchiveCancel: () => call("project_archive_cancel"),
    pickProjectArchivePath: (suggestedName) =>
      save({
        title: "Export portable project archive (choose a new filename)",
        defaultPath: `${suggestedName}.zip`,
        filters: [{ name: "Windfall project archive", extensions: ["zip"] }],
      }),
    recentProjects: () => call("recent_projects"),
    midiPreview: (path, options) => call("midi_preview", { path, options }),
    importMidi: (token) => call("import_midi", { token }),
    midiDiscard: (token) => call("midi_discard", { token }),
    exportMidi: (path, options) => call("export_midi", { path, options }),
    async pickMidiFile() {
      const result = await open({
        title: "Import MIDI",
        multiple: false,
        filters: MIDI_FILTER,
      })
      return typeof result === "string" ? result : null
    },
    pickMidiExportPath: (suggestedName) =>
      save({
        title: "Export MIDI",
        defaultPath: `${suggestedName}.mid`,
        filters: MIDI_FILTER,
      }),

    transportPlay: (guard) => call("transport_play", { guard }),
    transportStop: () => call("transport_stop"),
    transportToggle: () => call("transport_toggle"),
    transportSeek: (tick, guard) => call("transport_seek", { tick, guard }),
    transportSet: (patch, guard) => call("transport_set", { patch, guard }),
    transportState: () => call("transport_state"),

    engineStatus: () => call("engine_status"),
    recordingInputs: () => call("recording_inputs"),
    recordingState: () => call("recording_state"),
    recordingStart: (source, start, track) =>
      call("recording_start", { source, start, track }),
    recordingStop: () => call("recording_stop"),
    recordingCancel: () => call("recording_cancel"),
    engineDevices: () => call("engine_devices"),
    engineConfigure: (settings) => call("engine_configure", { settings }),
    engineSettings: () => call("engine_settings"),

    auditionNoteOn: (channel, key, velocity) =>
      call("audition_note_on", { channel, key, velocity }),
    auditionNoteOff: (channel, key) =>
      call("audition_note_off", { channel, key }),
    previewPlay: (path, browser) => call("preview_play", { path, browser }),
    previewStop: () => call("preview_stop"),

    browserRoots: () => call("browser_roots"),
    librarySearch: (search) => call("library_search", { search }),
    libraryRefresh: () => call("library_refresh"),
    libraryCancel: (generation) => call("library_cancel", { generation }),
    libraryFile: (path) => call("library_file", { path }),
    libraryMetadata: (path) => call("library_metadata", { path }),
    librarySetMetadata: (path, metadata) =>
      call("library_set_metadata", { path, metadata }),
    browserAddRoot: (path) => call("browser_add_root", { path }),
    browserRemoveRoot: (path) => call("browser_remove_root", { path }),
    browserList: (path) => call("browser_list", { path }),
    sampleInfo: (path, browser) => call("sample_info", { path, browser }),
    prepareClipCommand: (command) => call("prepare_clip_command", { command }),
    samplerPreparationBegin: () => call("sampler_preparation_begin"),
    samplerPreparationCancel: (request) =>
      call("sampler_preparation_cancel", { request }),
    samplerPreparationProgress: (request) =>
      call("sampler_preparation_progress", { request }),
    prepareSamplerCommand: (command, request) =>
      call("prepare_sampler_command", { command, request }),
    detectClipTempo: (sample) => call("detect_clip_tempo", { sample }),
    sampleInfoById: (sample) => call("sample_info_by_id", { sample }),
    addChannelFromFile: (path, index, browser) =>
      call("add_channel_from_file", { path, index, browser }),
    setChannelSampleFromFile: (channel, path, browser) =>
      call("set_channel_sample_from_file", { channel, path, browser }),
    addAudioClipFromFile: (path, place, browser) =>
      call("add_audio_clip_from_file", { path, ...place, browser }),
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
        filters: [
          {
            name: "Windfall project or archive",
            extensions: ["windfall", "zip"],
          },
        ],
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
