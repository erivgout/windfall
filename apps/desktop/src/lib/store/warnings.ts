import { create } from "zustand"

type WarningsState = {
  /** Problems the project loaded with, such as a missing sample file. */
  warnings: string[]
  /**
   * Goes up whenever the backend has read the sample files anew: when a
   * project loads and after a reload. What was read before is out of date.
   */
  samplesGeneration: number
}

/**
 * What is wrong with the project that is open. Set from the backend's
 * `project:warnings` event and emptied when another project loads.
 */
export const useWarningsStore = create<WarningsState>(() => ({
  warnings: [],
  samplesGeneration: 0,
}))

export function receiveWarnings(warnings: string[]) {
  useWarningsStore.setState({ warnings })
}

export function clearWarnings() {
  if (useWarningsStore.getState().warnings.length > 0) {
    useWarningsStore.setState({ warnings: [] })
  }
}

/** Says the backend has read the sample files anew. */
export function samplesReloaded() {
  useWarningsStore.setState((state) => ({
    samplesGeneration: state.samplesGeneration + 1,
  }))
}

export function useProjectWarnings(): string[] {
  return useWarningsStore((state) => state.warnings)
}
