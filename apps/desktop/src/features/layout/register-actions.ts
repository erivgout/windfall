import { registerBrowserActions } from "@/features/browser/actions"
import { registerChannelRackActions } from "@/features/channel-rack/actions"
import { registerMixerActions } from "@/features/mixer/actions"
import { registerPianoRollActions } from "@/features/piano-roll/actions"
import { registerPlaylistActions } from "@/features/playlist/actions"
import { registerBuiltinActions } from "@/lib/actions/builtin"

/**
 * Puts every action the app has in the registry: the shell's own and each
 * panel's. Returns a function that removes them again.
 */
export function registerAllActions(): () => void {
  const stops = [
    registerBuiltinActions(),
    registerBrowserActions(),
    registerChannelRackActions(),
    registerMixerActions(),
    registerPianoRollActions(),
    registerPlaylistActions(),
  ]
  return () => {
    for (const stop of stops) stop()
  }
}
