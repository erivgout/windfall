// Provisional source binding; regenerate during the deferred artifact pass.
import type { MeterChange } from "./MeterChange";
import type { TimeSignature } from "./TimeSignature";
import type { NoteGridUnit } from "./NoteGridUnit";
export type NoteMusicalGrid = { signature: TimeSignature; meters: Array<MeterChange>; unit: NoteGridUnit; divisor: number };
