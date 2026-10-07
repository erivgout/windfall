import { memo } from "react"

import type { Automation } from "@/bindings"
import { ContextActions } from "@/components/context-actions"
import { describeTarget } from "@/lib/automation/targets"
import { useHint } from "@/lib/store/hint"
import { useProjectStore } from "@/lib/store/project"
import { colorToCss } from "@/lib/units"

import { PICKER_ROW, PickerEmpty, PickerSection } from "../picker-section"
import { useAutomationClipCounts } from "../selectors"
import { usePlaylistStore } from "../store"
import { automationMenu } from "./menu"
import { renameAutomation } from "./ops"

const AutomationRow = memo(function AutomationRow({
  automation,
  target,
  selected,
  clips,
}: {
  automation: Automation
  target: string
  selected: boolean
  clips: number
}) {
  const setBrush = usePlaylistStore((state) => state.setBrush)
  const where =
    clips === 0
      ? "It has no clip yet, so it does nothing"
      : `${clips === 1 ? "One clip shows" : `${clips} clips show`} it, and an edit to one changes them all`
  const hint = useHint(
    `${automation.name} moves ${target}. ${where}. ${selected ? "Draw and Paint place more clips of it" : "Click to place clips of it"}. Double-click to rename, right-click for more`
  )

  return (
    <ContextActions items={() => automationMenu(automation)}>
      <button
        type="button"
        aria-pressed={selected}
        data-automation={automation.id}
        onClick={() =>
          setBrush({ type: "automation", automation: automation.id })
        }
        onDoubleClick={() => void renameAutomation(automation.id)}
        className={`${PICKER_ROW} h-9`}
        {...hint}
      >
        <span
          aria-hidden
          className="h-5 w-2 shrink-0 rounded-[2px]"
          style={{ backgroundColor: colorToCss(automation.color) }}
        />
        <span className="flex min-w-0 flex-1 flex-col leading-tight">
          <span className="truncate group-aria-pressed:font-medium">
            {automation.name}
          </span>
          <span className="truncate text-[0.625rem] text-muted-foreground">
            {target}
          </span>
        </span>
        {clips > 1 && (
          <span
            className="shrink-0 font-readout text-[0.625rem] text-muted-foreground"
            title={`${clips} clips show this curve`}
          >
            ×{clips}
          </span>
        )}
      </button>
    </ContextActions>
  )
})

/**
 * The automations of the project: each a curve that moves one thing. One
 * that is picked becomes the brush, which places more clips of the same
 * curve.
 */
export function AutomationSection() {
  const automations = useProjectStore((state) => state.project.automations)
  // The target's words follow the names of channels, tracks and effects.
  const channels = useProjectStore((state) => state.project.channels)
  const mixer = useProjectStore((state) => state.project.mixer)
  const settings = useProjectStore((state) => state.project.settings)
  const brush = usePlaylistStore((state) => state.brush)
  const counts = useAutomationClipCounts()
  const source = { channels, mixer, settings }

  return (
    <PickerSection title="Automation">
      {automations.length === 0 ? (
        <PickerEmpty>
          Right-click a knob or fader and choose Create automation clip.
        </PickerEmpty>
      ) : (
        <div
          role="group"
          aria-label="Automation to place"
          className="flex flex-col"
        >
          {automations.map((automation) => (
            <AutomationRow
              key={automation.id}
              automation={automation}
              target={describeTarget(source, automation.target)}
              selected={
                brush.type === "automation" &&
                brush.automation === automation.id
              }
              clips={counts.get(automation.id) ?? 0}
            />
          ))}
        </div>
      )}
    </PickerSection>
  )
}
