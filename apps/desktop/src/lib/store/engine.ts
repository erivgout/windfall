import { create } from "zustand"

import type { AudioHost, AudioSettings, EngineStatus } from "@/bindings"
import { reportError } from "@/lib/errors"
import { backend } from "@/lib/ipc"

type EngineState = {
  /** Null until the backend has reported once. */
  status: EngineStatus | null
  /** Audio drivers and their devices. Loaded when the settings open. */
  hosts: AudioHost[]
}

export const useEngineStore = create<EngineState>(() => ({
  status: null,
  hosts: [],
}))

export function receiveEngineStatus(status: EngineStatus) {
  useEngineStore.setState({ status })
}

export async function loadAudioDevices(): Promise<void> {
  try {
    useEngineStore.setState({ hosts: await backend.engineDevices() })
  } catch (error) {
    reportError(error, "Could not list audio devices")
  }
}

/** Opens the audio output the user asked for and reports how it went. */
export async function configureEngine(
  settings: AudioSettings
): Promise<EngineStatus | null> {
  try {
    const status = await backend.engineConfigure(settings)
    receiveEngineStatus(status)
    return status
  } catch (error) {
    reportError(error, "Could not change the audio device")
    return null
  }
}
