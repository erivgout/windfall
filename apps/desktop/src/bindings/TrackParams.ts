import type { EqBand } from "./EqBand";
// Provisional source binding; artifact generation is deferred.
export type TrackParams = { eqEnabled: boolean, low: EqBand, mid: EqBand, high: EqBand, invertLeft: boolean, invertRight: boolean, swap: boolean, separation: number };
