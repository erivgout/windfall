import { useEngineStore } from "@/lib/store"

/** Assumed while the engine has not reported, or has no device. */
export const FALLBACK_SAMPLE_RATE = 48_000

function usable(rate: number | undefined): number {
  return rate !== undefined && rate > 0 ? rate : FALLBACK_SAMPLE_RATE
}

/** The sample rate the engine runs at, which filters and latency follow. */
export function useSampleRate(): number {
  return useEngineStore((state) => usable(state.status?.sampleRate))
}

export function currentSampleRate(): number {
  return usable(useEngineStore.getState().status?.sampleRate)
}
