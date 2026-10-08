// Provisional source binding; regenerate with scripts/gen-bindings.sh in the deferred artifact pass.
import type { NoteArticulation } from "./NoteArticulation";
export type NoteExpression = { release: number, finePitchCents: number, modulationX: number, modulationY: number, articulation?: NoteArticulation, glideTicks?: number, colorGroup?: number | null };
