// Provisional source binding; regenerate during the deferred artifact pass.
import type { TimeSignature } from "./TimeSignature";
import type { MeterChange } from "./MeterChange";
import type { MeterChangeId } from "./MeterChangeId";
import type { TimelineMarker } from "./TimelineMarker";
import type { TimelineMarkerId } from "./TimelineMarkerId";
export type PatternTimelineEdit = { type: "setSignature", signature: TimeSignature | null } | { type: "addMeter", tick: number, signature: TimeSignature } | { type: "updateMeter", change: MeterChange } | { type: "removeMeter", id: MeterChangeId } | { type: "addMarker", tick: number, name: string } | { type: "updateMarker", marker: TimelineMarker } | { type: "removeMarker", id: TimelineMarkerId };
