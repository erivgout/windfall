import { reportError } from "@/lib/errors"
import { backend } from "@/lib/ipc"

import { receiveEngineStatus } from "./engine"
import { loadSnapshot, receivePatch, refetchSnapshot } from "./project"
import { receiveTransportState } from "./transport"
import { useUiStore } from "./ui"

/**
 * Wires the stores to the backend: loads the current state and keeps it up
 * to date from the backend's events. Call once per window. Returns a
 * function that disconnects.
 */
export function connectStores(): () => void {
  const disconnect = [
    backend.onProjectPatch(receivePatch),
    backend.onProjectLoaded((snapshot) => {
      loadSnapshot(snapshot)
      // Ids from the previous project mean nothing in the new one.
      useUiStore.setState({ selectedChannel: null, selectedTrack: null })
    }),
    backend.onTransportState(receiveTransportState),
    backend.onEngineStatus(receiveEngineStatus),
    backend.onProjectWarnings((warnings) => {
      for (const warning of warnings) reportError(warning, "Project")
    }),
  ]

  void refetchSnapshot()
  backend
    .transportState()
    .then(receiveTransportState)
    .catch((error: unknown) =>
      reportError(error, "Could not read the transport")
    )
  backend
    .engineStatus()
    .then(receiveEngineStatus)
    .catch((error: unknown) =>
      reportError(error, "Could not read the audio engine")
    )

  return () => {
    for (const off of disconnect) off()
  }
}
