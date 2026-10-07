// IPC contract for this feature, matching windfall-ipc/src/audio_edit.rs.
// Kept outside generated bindings until the parent's combined regeneration.
export type AudioEditPreview = {
  token: number
  clip: number
  name: string
  frames: number
  sampleRate: number
  peaks: number[]
}
export type AudioEditOperation =
  | "trim"
  | "extract"
  | "normalize"
  | "reverse"
  | "fadeIn"
  | "fadeOut"
  | "silence"
  | "cut"
export type AudioEditRequest = {
  token: number
  operation: AudioEditOperation
  startFrame: number
  endFrame: number
}
