import type { MixerRecordMode } from "./MixerRecordMode";
import type { AudioInputRoute } from "./AudioInputRoute";
export type MixerRecording = { input: AudioInputRoute | null, armed: boolean, monitor: boolean, monitorGain: number, monitorBufferMs: number, offsetMs: number, mode?: MixerRecordMode };
