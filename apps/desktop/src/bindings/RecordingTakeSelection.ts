export type RecordingTakeSelection = { type: "all" } | { type: "latest" } | { type: "only", indices: number[] } | { type: "except", indices: number[] };
