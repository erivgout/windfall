import { useEffect } from "react"

import { ContextActions } from "@/components/context-actions"
import { useShortcutScope } from "@/lib/actions"

import { BrowserHeader } from "./browser-header"
import { flushScrollTop } from "./persist"
import { PreviewPane } from "./preview-pane"
import { BROWSER_MENU } from "./row-menu"
import { ensureRootsLoaded } from "./store"
import { TreeView } from "./tree-view"

/**
 * The browser: find a sound, hear it at once, and get it into the rack.
 * Tools on top, the tree of folders in the middle, the selected sound below.
 * Its state lives in a store of its own, so hiding the panel loses nothing.
 */
export default function BrowserPanel() {
  useEffect(() => {
    ensureRootsLoaded()
    // The scroll position is saved a moment after scrolling stops; make sure
    // the last one is not lost when the panel or the window goes away.
    window.addEventListener("pagehide", flushScrollTop)
    return () => {
      window.removeEventListener("pagehide", flushScrollTop)
      flushScrollTop()
    }
  }, [])

  const scope = useShortcutScope("browser")

  return (
    <ContextActions items={BROWSER_MENU}>
      <div
        data-slot="browser-panel"
        className="@container/browser flex h-full min-h-0 flex-col"
        {...scope}
      >
        <BrowserHeader />
        <TreeView />
        <PreviewPane />
      </div>
    </ContextActions>
  )
}
