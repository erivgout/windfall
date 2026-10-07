import { create } from "zustand"

import type { EffectId } from "@/bindings"
import { onProjectReplaced } from "@/lib/store/replaced"

import type { EqBandId } from "./eq-response"
import { DEFAULT_DB_RANGE, type DbRange } from "./geometry"

type EqViewState = {
  /** The band whose controls show under the display, per equaliser. */
  band: Record<EffectId, EqBandId>
  /** The gain range the display shows, per equaliser. */
  range: Record<EffectId, DbRange>
}

/**
 * How each equaliser's editor is looked at. It lives outside the editor so
 * folding an effect away and opening it again changes nothing.
 */
export const useEqView = create<EqViewState>(() => ({ band: {}, range: {} }))

// Effect ids start over in every project.
onProjectReplaced(() => useEqView.setState({ band: {}, range: {} }))

export function useEqBand(effect: EffectId): EqBandId {
  return useEqView((state) => state.band[effect] ?? "peak2")
}

export function useEqRange(effect: EffectId): DbRange {
  return useEqView((state) => state.range[effect] ?? DEFAULT_DB_RANGE)
}

export function selectEqBand(effect: EffectId, band: EqBandId) {
  useEqView.setState((state) =>
    state.band[effect] === band
      ? state
      : { band: { ...state.band, [effect]: band } }
  )
}

export function setEqRange(effect: EffectId, range: DbRange) {
  useEqView.setState((state) => ({
    range: { ...state.range, [effect]: range },
  }))
}
