import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"
import { Kbd } from "@/components/ui/kbd"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { runAction, useShortcutLabel } from "@/lib/actions"
import { useChannelCount, useProjectReady } from "@/lib/store/selectors"
import { useUiStore, type CenterTab, type PanelSizes } from "@/lib/store/ui"
import { cn } from "@/lib/utils"

import { EmptyProject } from "./empty-states"
import { PanelBoundary, PanelFrame } from "./panel-frame"
import { CENTER_TABS, PANELS } from "./panels"

/**
 * Sizes are saved per group and per set of visible panels, so the workspace
 * with the mixer hidden remembers a different split than with it showing.
 */
function useSavedLayout(group: string, panels: string[]) {
  const key = `${group}:${panels.join("+")}`
  const saved = useUiStore((state) => state.layouts[key])
  const saveLayout = useUiStore((state) => state.saveLayout)
  return {
    defaultLayout: saved,
    onLayoutChanged: (sizes: PanelSizes) => saveLayout(key, sizes),
  }
}

function CenterTabButton({ tab }: { tab: CenterTab }) {
  const active = useUiStore((state) => state.centerTab === tab)
  const { title, action } = PANELS[tab]
  const shortcut = useShortcutLabel(action)

  return (
    <Tooltip>
      <TooltipTrigger
        role="tab"
        id={`center-tab-${tab}`}
        aria-selected={active}
        aria-controls="center-panel"
        tabIndex={active ? 0 : -1}
        onClick={() => void runAction(action)}
        className={cn(
          "relative flex h-7 items-center px-2.5 text-xs text-muted-foreground outline-none hover:text-foreground focus-visible:bg-accent focus-visible:text-foreground",
          active &&
            "font-medium text-foreground after:absolute after:inset-x-2.5 after:bottom-0 after:h-0.5 after:rounded-t-full after:bg-brand"
        )}
      >
        {title}
      </TooltipTrigger>
      <TooltipContent side="bottom">
        Show the {title.toLowerCase()}
        {shortcut && <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  )
}

/** The middle of the window: channel rack, playlist and piano roll as tabs. */
function CenterDock() {
  const tab = useUiStore((state) => state.centerTab)
  const ready = useProjectReady()
  const channelCount = useChannelCount()
  const { title, component: Panel } = PANELS[tab]
  const showStart = tab === "channelRack" && ready && channelCount === 0

  function onKeyDown(event: React.KeyboardEvent) {
    const step =
      event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0
    if (step === 0) return
    const next =
      CENTER_TABS[
        (CENTER_TABS.indexOf(tab) + step + CENTER_TABS.length) %
          CENTER_TABS.length
      ]
    void runAction(PANELS[next].action)
    document.getElementById(`center-tab-${next}`)?.focus()
  }

  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col bg-background">
      <div
        role="tablist"
        aria-label="Editors"
        onKeyDown={onKeyDown}
        className="flex h-7 shrink-0 items-center border-b bg-chassis/60 px-1"
      >
        {CENTER_TABS.map((item) => (
          <CenterTabButton key={item} tab={item} />
        ))}
      </div>
      <div
        role="tabpanel"
        id="center-panel"
        aria-labelledby={`center-tab-${tab}`}
        className="min-h-0 flex-1 overflow-auto"
      >
        <PanelBoundary key={tab} name={title.toLowerCase()}>
          {showStart ? <EmptyProject /> : <Panel />}
        </PanelBoundary>
      </div>
    </div>
  )
}

function SideDock({ panel }: { panel: "browser" | "mixer" }) {
  const { title, action, component: Panel } = PANELS[panel]
  return (
    <PanelFrame title={title} hideAction={action}>
      <Panel />
    </PanelFrame>
  )
}

function MainColumn() {
  const mixer = useUiStore((state) => state.panels.mixer)
  const layout = useSavedLayout(
    "main",
    mixer ? ["center", "mixer"] : ["center"]
  )

  return (
    <ResizablePanelGroup id="main" orientation="vertical" {...layout}>
      <ResizablePanel id="center" minSize={140}>
        <CenterDock />
      </ResizablePanel>
      {mixer && (
        <>
          <ResizableHandle aria-label="Resize the mixer" />
          <ResizablePanel
            id="mixer"
            defaultSize="38%"
            minSize={120}
            maxSize="75%"
          >
            <SideDock panel="mixer" />
          </ResizablePanel>
        </>
      )}
    </ResizablePanelGroup>
  )
}

/**
 * The docked panels between the transport and the status bar: the browser on
 * the left, the editors in the middle and the mixer below them. Each one can
 * be resized and hidden, and the arrangement is remembered.
 */
export function Workspace() {
  const browser = useUiStore((state) => state.panels.browser)
  const generation = useUiStore((state) => state.layoutGeneration)
  const layout = useSavedLayout(
    "workspace",
    browser ? ["browser", "main"] : ["main"]
  )

  return (
    <ResizablePanelGroup
      // "Reset layout" bumps the generation, which rebuilds the groups from
      // their default sizes.
      key={generation}
      id="workspace"
      orientation="horizontal"
      className="min-h-0 flex-1"
      {...layout}
    >
      {browser && (
        <>
          <ResizablePanel
            id="browser"
            defaultSize="19%"
            minSize={180}
            maxSize="45%"
          >
            <SideDock panel="browser" />
          </ResizablePanel>
          <ResizableHandle aria-label="Resize the browser" />
        </>
      )}
      <ResizablePanel id="main" minSize={360}>
        <MainColumn />
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}
