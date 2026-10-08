import type { NoteMusicalGrid } from "./NoteMusicalGrid";
import type { CurveLfo } from "./CurveLfo";
import type { NoteLfoProperty } from "./NoteLfoProperty";
// Provisional source binding; regenerate during the deferred artifact pass.
import type { ArpDirection } from "./ArpDirection";
import type { ChopStep } from "./ChopStep";
import type { FlamPosition } from "./FlamPosition";
import type { NoteEdge } from "./NoteEdge";
import type { NoteGroove } from "./NoteGroove";
import type { RhythmMode } from "./RhythmMode";

export type NoteTransform = { type: "lfo", property: NoteLfoProperty, origin: number, strength: number, lfo: CurveLfo } | { "type": "randomize", seed: number, pitch: number, velocity: number, pan: number, timing: number, length: number } | { "type": "generateRandom", seed: number, grid: number, density: number, gate: number, root: number, pitchClasses: number, low: number, high: number, velocityLow: number, velocityHigh: number } | { "type": "legato" } | { "type": "staccato", factor: number, } | { "type": "chop", grid: number, } | { "type": "chopPattern", origin: number, period: number, steps: Array<ChopStep>, } | { "type": "arpeggiate", rate: number, gate: number, octaves: number, repetitions: number, direction: ArpDirection, } | { "type": "flam", interval: number, velocity: number, position: FlamPosition, } | { "type": "rhythmReshape", origin: number, step: number, period: number, phase: number, offset: number, mode: RhythmMode, } | { "type": "glue" } | { "type": "strum", spacing: number, velocityStep: number, descending: boolean, } | { "type": "flipTime" } | { "type": "flipPitch" } | { "type": "keyRange", low: number, high: number, transpose: number, octaves: boolean, } | { "type": "scaleVelocity", factor: number, } | { "type": "quantize", musical?: NoteMusicalGrid | null, grid: number, strength: number, edge: NoteEdge, groove: NoteGroove, };
