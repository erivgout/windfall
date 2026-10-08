import { registerAnalysisActions } from "@/features/analysis/actions"
import { registerBrowserActions } from "@/features/browser/actions"
import { registerChannelRackActions } from "@/features/channel-rack/actions"
import { registerMixerActions } from "@/features/mixer/actions"
import { registerPianoRollActions } from "@/features/piano-roll/actions"
import { registerPlaylistActions } from "@/features/playlist/actions"
import { registerBuiltinActions } from "@/lib/actions/builtin"
import { registerPluginActions } from "@/features/plugins/store"

/**
 * Puts every action the app has in the registry: the shell's own and each
 * panel's. Returns a function that removes them again.
 */
export function registerAllActions(): () => void {
  const stops = [
    registerBuiltinActions(),
    registerPluginActions(),
    registerBrowserActions(),
    registerChannelRackActions(),
    registerMixerActions(),
    registerPianoRollActions(),
    registerPlaylistActions(),
    registerAnalysisActions(),
  ]
  return () => {
    for (const stop of stops) stop()
  }
}
