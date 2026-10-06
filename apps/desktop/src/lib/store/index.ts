/*
 * Everything a panel needs from the stores, in one import:
 *
 *     import { dispatch, useChannel, useGesture, useMeter } from "@/lib/store"
 */
export {
  dispatch,
  historyJump,
  redo,
  undo,
  useProjectStore,
  type ProjectState,
} from "./project"
export * from "./selectors"
export {
  play,
  seek,
  setPlayMode,
  setTransport,
  setTransportPattern,
  stop,
  togglePlayback,
  useTransportStore,
} from "./transport"
export { configureEngine, loadAudioDevices, useEngineStore } from "./engine"
export {
  resolveTheme,
  useUiStore,
  type CenterTab,
  type KeymapPreset,
  type SidePanel,
  type Theme,
} from "./ui"
export {
  meterFeed,
  realtimeFrame,
  subscribeRealtime,
  useMeter,
  usePlayhead,
  useRealtime,
} from "./realtime"
export { newGestureId, useGesture, type Gesture } from "./gesture"
export { useHint } from "./hint"
export { askConfirm, askText } from "./prompts"
