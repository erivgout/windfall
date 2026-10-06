import { create } from "zustand"

import type {
  PatternId,
  PlayMode,
  TransportPatch,
  TransportState,
} from "@/bindings"
import { reportError } from "@/lib/errors"
import { backend } from "@/lib/ipc"

/** What the transport is doing, as last reported by the backend. */
export const useTransportStore = create<TransportState>(() => ({
  playing: false,
  mode: "pattern",
  pattern: 0,
  loopSong: true,
}))

export function receiveTransportState(state: TransportState) {
  useTransportStore.setState(state)
}

async function run(work: Promise<TransportState>) {
  try {
    receiveTransportState(await work)
  } catch (error) {
    reportError(error)
  }
}

export function play(): Promise<void> {
  return run(backend.transportPlay())
}

export function stop(): Promise<void> {
  return run(backend.transportStop())
}

export function togglePlayback(): Promise<void> {
  return run(backend.transportToggle())
}

export function setTransport(patch: TransportPatch): Promise<void> {
  return run(backend.transportSet(patch))
}

export function setPlayMode(mode: PlayMode): Promise<void> {
  return setTransport({ mode })
}

/** Selects the pattern that is edited and that plays in pattern mode. */
export function setTransportPattern(pattern: PatternId): Promise<void> {
  return setTransport({ pattern })
}

export async function seek(tick: number): Promise<void> {
  try {
    await backend.transportSeek(tick)
  } catch (error) {
    reportError(error)
  }
}
