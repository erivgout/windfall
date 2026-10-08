import type { RecordingAlignmentStatus } from "./RecordingAlignmentStatus";
import type { RecordingMonitorStatus } from "./RecordingMonitorStatus";
import type { TrackId } from "./TrackId";
export type RecordingTrackInfo = { mixerTrack: TrackId, name: string, frames: number, alignment: RecordingAlignmentStatus, monitor: RecordingMonitorStatus | null };
