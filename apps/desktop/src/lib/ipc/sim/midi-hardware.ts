import type { MidiHardwareState } from "@/bindings"
import type { Backend } from "../backend"

export function createMidiHardwareMock(): Pick<
  Backend,
  | "midiHardwareState"
  | "midiHardwareRefresh"
  | "midiHardwareConfigure"
  | "midiHardwareTarget"
  | "midiHardwarePanic"
> {
  const state = async (): Promise<MidiHardwareState> => ({
    settings: {
      input: null,
      output: null,
      inputChannel: null,
      outputChannel: 1,
    },
    inputs: [],
    outputs: [],
    inputConnected: false,
    outputConnected: false,
    target: null,
    generation: 0,
    droppedEvents: 0,
    error:
      "MIDI hardware is available in the desktop app. Browser simulation has no native MIDI ports.",
  })
  const unavailable = async (): Promise<never> => {
    throw new Error("MIDI hardware requires the desktop app.")
  }
  return {
    midiHardwareState: state,
    midiHardwareRefresh: state,
    midiHardwareConfigure: unavailable,
    midiHardwareTarget: unavailable,
    midiHardwarePanic: async () => {},
  }
}
