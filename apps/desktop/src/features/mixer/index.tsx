import { logicalWheel } from "@/lib/ui-scale"
import { Add01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { useEffect, useRef, useState, type WheelEvent } from "react"

import type { TrackId } from "@/bindings"
import { ActionButton } from "@/components/action-button"
import { ContextActions } from "@/components/context-actions"
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"
import { useShortcutScope } from "@/lib/actions"
import { useMixerTrackIds, useProjectStore, useUiStore } from "@/lib/store"
import type { PanelSizes } from "@/lib/store/ui"
import { MASTER_TRACK } from "@/lib/units"

import { registerMixerActions } from "./actions"
import { useEffectsUi } from "./effects-ui"
import { EffectInspector, EffectInspectorTab } from "./inspector"
import {
  ADD_WIDTH,
  INSPECTOR_MIN_WIDTH,
  inspectorWidthFor,
  MASTER_WIDTH,
  STRIP_WIDTH,
  stripLayout,
  visibleRange,
} from "./layout"
import { MIXER_MENU } from "./menus"
import { useMixerUi } from "./mixer-ui"
import { watchPeaks } from "./peaks"
import { maxSendCount } from "./routing"
import { MixerStrip } from "./strip"
import { maxEffectCount } from "./strip-track"
import { useStripView } from "./strip-view"

/** Where the split between the strips and the effects is remembered. */
const LAYOUT_KEY = "mixer:strips+effects"

/** Assumed until the panel has been measured. */
const DEFAULT_VIEW_WIDTH = 1280

/** The mixer track the channel selected in the rack plays into. */
function useLinkedTrack(): TrackId | null {
  const channel = useUiStore((state) => state.selectedChannel)
  return useProjectStore(
    (state) =>
      state.project.channels.find((item) => item.id === channel)?.mixerTrack ??
      null
  )
}

function AddTrack({ first }: { first: boolean }) {
  return (
    <div className="flex h-full items-start gap-3 p-1.5">
      <ActionButton
        action="mixer.addTrack"
        variant="outline"
        size="icon-sm"
        tooltipSide="top"
        className="border-dashed text-muted-foreground hover:text-foreground"
      >
        <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
      </ActionButton>
      {first && (
        <p className="max-w-64 pt-0.5 leading-snug text-muted-foreground">
          No insert tracks yet. Every channel you add to the rack gets its own
          track here, or add an empty one to use as a bus.
        </p>
      )}
    </div>
  )
}

/**
 * The mixer: the master pinned on the left, then one strip per insert
 * track, scrolling sideways, and the selected track's effects docked on
 * the right.
 *
 * Only the strips in view are mounted. A strip is a fader, a knob, a canvas
 * and a dozen store subscriptions, and a full mixer has 128 of them; mounting
 * the few on screen keeps resizing, scrolling and metering at the cost of
 * what is visible, however long the mixer gets. Peaks and clips of tracks
 * that are out of view are still held (see `peaks.ts`).
 */
export default function MixerPanel() {
  const ids = useMixerTrackIds()
  const selected = useUiStore((state) => state.selectedTrack)
  const linked = useLinkedTrack()
  const maxSends = useProjectStore((state) =>
    maxSendCount(state.project.mixer.tracks)
  )
  const maxEffects = useProjectStore((state) =>
    maxEffectCount(state.project.mixer.tracks)
  )
  const enlarged = useUiStore((state) => state.centerOverlay === "effects")
  // Enlarged, the effects are in the editor area and not in here.
  const inspectorOpen =
    useEffectsUi((state) => state.inspectorOpen) && !enlarged
  // Once the divider has been dragged, the width it was left at is used.
  const [inspectorWidth] = useState(() => inspectorWidthFor(window.innerWidth))
  const savedLayout = useUiStore((state) => state.layouts[LAYOUT_KEY])
  const saveLayout = useUiStore((state) => state.saveLayout)
  const { view, attach, reveal } = useStripView()
  const root = useRef<HTMLDivElement>(null)
  const focusing = useMixerUi((state) => state.focusing)
  const scope = useShortcutScope("mixer")

  useEffect(() => registerMixerActions(), [])
  useEffect(() => watchPeaks(), [])

  const inserts = ids.filter((id) => id !== MASTER_TRACK)
  const { mode, sendRows, effectRows } = stripLayout(
    view.height,
    maxSends,
    maxEffects
  )
  const range = visibleRange(
    view.offset,
    // The offset is rounded down to a whole strip, so look one further.
    (view.width || DEFAULT_VIEW_WIDTH) + STRIP_WIDTH,
    inserts.length
  )
  const selectedIndex = selected === null ? -1 : inserts.indexOf(selected)
  const linkedIndex = linked === null ? -1 : inserts.indexOf(linked)

  // The selected strip stays mounted when it scrolls away, so the control
  // that has the focus is never pulled out from under the keyboard.
  const mounted: number[] = []
  for (let index = range.start; index < range.end; index += 1) {
    mounted.push(index)
  }
  if (selectedIndex >= 0 && !mounted.includes(selectedIndex)) {
    mounted.push(selectedIndex)
    mounted.sort((a, b) => a - b)
  }

  useEffect(() => {
    if (selectedIndex >= 0) reveal(selectedIndex)
  }, [selected, selectedIndex, reveal])

  // A key that moves the selection takes the focus along. The strip may
  // only just have been mounted, so this waits for it.
  useEffect(() => {
    if (focusing === null) return
    const strip = root.current?.querySelector<HTMLElement>(
      `[data-track="${focusing}"]`
    )
    if (!strip) return
    strip.focus({ preventScroll: true })
    useMixerUi.setState({ focusing: null })
  }, [focusing, selected])

  useEffect(() => {
    if (linkedIndex >= 0) reveal(linkedIndex)
  }, [linked, linkedIndex, reveal])

  // A plain mouse wheel has nowhere to go here, so it moves along the
  // strips. A control that took the wheel for itself has claimed the event.
  function onWheel(event: WheelEvent<HTMLDivElement>) {
    if (event.defaultPrevented || event.ctrlKey) return
    if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return
    const target = event.target
    if (!(target instanceof Element)) return
    if (!event.currentTarget.contains(target)) return
    const list = target.closest("ul")
    if (list && list.scrollHeight > list.clientHeight) return
    event.currentTarget.scrollLeft += logicalWheel(event, {
      line: 1,
      page: 1,
    }).deltaY
  }

  return (
    <ContextActions items={MIXER_MENU}>
      <div
        ref={root}
        data-slot="mixer"
        data-mode={mode}
        className="flex h-full min-h-0 min-w-0"
        {...scope}
      >
        <ResizablePanelGroup
          id="mixer"
          orientation="horizontal"
          className="min-w-0 flex-1"
          defaultLayout={inspectorOpen ? savedLayout : undefined}
          onLayoutChanged={(sizes: PanelSizes) => {
            if (inspectorOpen) saveLayout(LAYOUT_KEY, sizes)
          }}
        >
          <ResizablePanel
            id="strips"
            minSize={MASTER_WIDTH + 2 * STRIP_WIDTH}
            className="flex min-w-0"
          >
            <div
              data-slot="mixer-master"
              className="z-10 shrink-0 border-r bg-chassis shadow-[2px_0_6px_-2px_rgb(0_0_0/0.35)]"
              style={{ width: MASTER_WIDTH, paddingBottom: view.scrollbar }}
            >
              <MixerStrip
                id={MASTER_TRACK}
                mode={mode}
                sendRows={sendRows}
                effectRows={effectRows}
                linked={linked === MASTER_TRACK}
                metering
              />
            </div>
            <div
              ref={attach}
              role="group"
              aria-label="Insert tracks"
              data-slot="mixer-inserts"
              className="min-w-0 flex-1 overflow-x-auto overflow-y-hidden"
              onWheel={onWheel}
            >
              <div
                className="relative h-full"
                style={{
                  width:
                    inserts.length > 0
                      ? inserts.length * STRIP_WIDTH + ADD_WIDTH
                      : undefined,
                }}
              >
                {mounted.map((index) => (
                  <div
                    key={inserts[index]}
                    className="absolute inset-y-0"
                    style={{ left: index * STRIP_WIDTH, width: STRIP_WIDTH }}
                  >
                    <MixerStrip
                      id={inserts[index]}
                      mode={mode}
                      sendRows={sendRows}
                      effectRows={effectRows}
                      linked={inserts[index] === linked}
                      metering={index >= range.start && index < range.end}
                    />
                  </div>
                ))}
                <div
                  className="absolute inset-y-0"
                  style={{ left: inserts.length * STRIP_WIDTH }}
                >
                  <AddTrack first={inserts.length === 0} />
                </div>
              </div>
            </div>
          </ResizablePanel>
          {inspectorOpen && (
            <>
              <ResizableHandle aria-label="Resize the effects" />
              <ResizablePanel
                id="effects"
                defaultSize={inspectorWidth}
                minSize={INSPECTOR_MIN_WIDTH}
                maxSize="70%"
                groupResizeBehavior="preserve-pixel-size"
              >
                <EffectInspector />
              </ResizablePanel>
            </>
          )}
        </ResizablePanelGroup>
        {!inspectorOpen && <EffectInspectorTab enlarged={enlarged} />}
      </div>
    </ContextActions>
  )
}
