import { create } from "zustand"
import { toast } from "sonner"
import type { RecordingInput, RecordingState } from "@/bindings"
import { backend, errorMessage } from "@/lib/ipc"
import { receivePatch } from "@/lib/store/project"

export const useRecordingStore = create(() => ({
  open: false,
  busy: false,
  inputs: [] as RecordingInput[],
  device: 0,
  left: 0,
  right: -1,
  start: 0,
  track: -1,
  state: {
    active: false,
    frames: 0,
    sampleRate: 0,
    startTick: 0,
    error: null,
  } as RecordingState,
  error: "",
}))
async function work(action: () => Promise<void>) {
  useRecordingStore.setState({ busy: true, error: "" })
  try {
    await action()
  } catch (error) {
    useRecordingStore.setState({ error: errorMessage(error) })
  } finally {
    useRecordingStore.setState({ busy: false })
    await refreshRecording()
  }
}
export async function refreshRecording() {
  try {
    useRecordingStore.setState({ state: await backend.recordingState() })
  } catch (error) {
    useRecordingStore.setState({ error: errorMessage(error) })
  }
}
export async function openRecording() {
  useRecordingStore.setState({ open: true })
  await work(async () => {
    useRecordingStore.setState({ inputs: await backend.recordingInputs() })
  })
}
export function startRecording() {
  return work(async () => {
    const s = useRecordingStore.getState()
    const device = s.inputs[s.device]
    if (!device) throw new Error("Select a native audio input.")
    const state = await backend.recordingStart(
      {
        host: device.host,
        device: device.device,
        left: s.left,
        right: s.right < 0 ? null : s.right,
      },
      s.start,
      s.track < 0 ? null : s.track
    )
    useRecordingStore.setState({ state })
  })
}
export function stopRecording() {
  return work(async () => {
    const result = await backend.recordingStop()
    receivePatch(result.patch)
    toast.success("Recording added to playlist")
  })
}
export function cancelRecording() {
  return work(async () => {
    await backend.recordingCancel()
    toast("Recording discarded")
  })
}
