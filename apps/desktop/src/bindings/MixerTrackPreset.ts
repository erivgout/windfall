// Provisionally synchronized from Rust; regeneration is deferred.
export type MixerTrackPreset = {
  version: number; name: string; color: number; volume: number; pan: number; muted: boolean;
  processing: import("./TrackParams").TrackParams; latencyOffsetMs: number;
  effects: import("./EffectSlot").EffectSlot[]; plugins: import("./PluginBinding").PluginBinding[];
}
