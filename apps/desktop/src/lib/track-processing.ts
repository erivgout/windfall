import type { ParamInfo, TrackParams } from "@/bindings"

export const DEFAULT_TRACK_PROCESSING: TrackParams = {
  eqEnabled: true,
  low: { enabled: true, frequencyHz: 100, gainDb: 0, q: 0.70710677 },
  mid: { enabled: true, frequencyHz: 1000, gainDb: 0, q: 1 },
  high: { enabled: true, frequencyHz: 8000, gainDb: 0, q: 0.70710677 },
  invertLeft: false, invertRight: false, swap: false, separation: 0,
}
const toggle = (id: string, name: string, value: boolean): ParamInfo => ({ id, name, kind: "toggle", unit: "none", scale: "linear", min: 0, max: 1, default: value ? 1 : 0, choices: [] })
const float = (id: string, name: string, min: number, max: number, value: number, unit: ParamInfo["unit"] = "none", scale: ParamInfo["scale"] = "linear"): ParamInfo => ({ id, name, min, max, default: value, unit, scale, kind: "float", choices: [] })
// Provisional mirror of TrackParams::descriptors; indices are persisted.
export const TRACK_PROCESSING_INFO: readonly ParamInfo[] = [
  toggle("eqEnabled", "Track EQ", true),
  toggle("low.enabled", "Low shelf", true),
  float("low.frequencyHz", "Low frequency", 20, 20000, 100, "hertz", "logarithmic"),
  float("low.gainDb", "Low gain", -24, 24, 0, "decibels"),
  float("low.q", "Low Q", 0.1, 2, 0.70710677, "none", "logarithmic"),
  toggle("mid.enabled", "Mid bell", true),
  float("mid.frequencyHz", "Mid frequency", 20, 20000, 1000, "hertz", "logarithmic"),
  float("mid.gainDb", "Mid gain", -24, 24, 0, "decibels"),
  float("mid.q", "Mid Q", 0.1, 18, 1, "none", "logarithmic"),
  toggle("high.enabled", "High shelf", true),
  float("high.frequencyHz", "High frequency", 20, 20000, 8000, "hertz", "logarithmic"),
  float("high.gainDb", "High gain", -24, 24, 0, "decibels"),
  float("high.q", "High Q", 0.1, 2, 0.70710677, "none", "logarithmic"),
  toggle("invertLeft", "Invert left polarity", false),
  toggle("invertRight", "Invert right polarity", false),
  toggle("swap", "Swap left and right", false),
  float("separation", "Stereo separation", -1, 1, 0),
]
