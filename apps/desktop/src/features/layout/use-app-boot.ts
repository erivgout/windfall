import { useEffect } from "react"

import { registerBrowserActions } from "@/features/browser/actions"
import { registerChannelRackActions } from "@/features/channel-rack/actions"
import { registerMixerActions } from "@/features/mixer/actions"
import { installKeymap } from "@/lib/actions"
import {
  registerBuiltinActions,
  syncRecentActions,
} from "@/lib/actions/builtin"
import {
  confirmDiscardChanges,
  refreshRecentProjects,
  useRecentStore,
} from "@/lib/flows/project"
import { backend, errorMessage } from "@/lib/ipc"
import { connectStores } from "@/lib/store/connect"
import { useDirty, useProjectName } from "@/lib/store/selectors"

type BootOptions = {
  /** The main window asks before closing with unsaved edits. */
  guardClose: boolean
}

/**
 * Starts everything a window needs: the stores follow the backend, the
 * actions are registered and the keymap listens. Call once at the top of a
 * window.
 */
export function useAppBoot({ guardClose }: BootOptions) {
  useEffect(() => {
    const stop = [
      connectStores(),
      registerBuiltinActions(),
      registerBrowserActions(),
      registerChannelRackActions(),
      registerMixerActions(),
      installKeymap(),
      useRecentStore.subscribe((state) => syncRecentActions(state.paths)),
    ]
    if (guardClose) stop.push(backend.onCloseRequested(confirmDiscardChanges))
    void refreshRecentProjects()
    return () => {
      for (const off of stop) off()
    }
  }, [guardClose])
}

/** Puts the project name in the window title, with a mark for unsaved edits. */
export function useWindowTitle() {
  const name = useProjectName()
  const dirty = useDirty()

  useEffect(() => {
    const title = `${name || "Untitled"}${dirty ? "*" : ""} - Windfall`
    // Not something the user did or can fix, so it is logged, not shown.
    backend.setWindowTitle(title).catch((error: unknown) => {
      console.warn("Could not set the window title:", errorMessage(error))
    })
  }, [name, dirty])
}
