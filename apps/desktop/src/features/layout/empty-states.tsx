import {
  GridViewIcon,
  Playlist01Icon,
  MusicNote03Icon,
} from "@hugeicons/core-free-icons"
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react"

import { ActionButton } from "@/components/action-button"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Kbd } from "@/components/ui/kbd"
import { useShortcutLabel } from "@/lib/actions"
import { useUiStore } from "@/lib/store/ui"

function Step({ number, children }: { number: number; children: string }) {
  return (
    <li className="flex items-baseline gap-2.5 text-left">
      <span className="w-3 shrink-0 text-right font-readout text-brand">
        {number}
      </span>
      <span>{children}</span>
    </li>
  )
}

/**
 * What a project with no channels shows in place of the channel rack: the
 * first three things to do, with buttons that do the first one.
 */
export function EmptyProject() {
  const browserVisible = useUiStore((state) => state.panels.browser)
  const playShortcut = useShortcutLabel("transport.toggle")

  return (
    <Empty className="h-full">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <HugeiconsIcon icon={GridViewIcon} strokeWidth={2} />
        </EmptyMedia>
        <EmptyTitle>This project has no sounds yet</EmptyTitle>
        <EmptyDescription>
          A beat starts with a channel: one sound and a row of steps.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <ol className="flex flex-col gap-1.5 text-muted-foreground">
          <Step number={1}>
            Add a channel, or drag a sample in from the browser.
          </Step>
          <Step number={2}>Click steps in its row to place hits.</Step>
          <li className="flex items-baseline gap-2.5 text-left">
            <span className="w-3 shrink-0 text-right font-readout text-brand">
              3
            </span>
            <span className="flex items-center gap-1.5">
              Press {playShortcut ? <Kbd>{playShortcut}</Kbd> : "Play"} to hear
              it loop.
            </span>
          </li>
        </ol>
        <div className="mt-2 flex flex-wrap justify-center gap-2">
          <ActionButton action="channel.add" />
          {!browserVisible && (
            <ActionButton action="view.browser" variant="outline">
              Show the browser
            </ActionButton>
          )}
          <ActionButton action="file.open" variant="outline" />
        </div>
      </EmptyContent>
    </Empty>
  )
}

type LaterPanelProps = {
  icon: IconSvgElement
  title: string
  description: string
}

function LaterPanel({ icon, title, description }: LaterPanelProps) {
  return (
    <Empty className="h-full">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <HugeiconsIcon icon={icon} strokeWidth={2} />
        </EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <ActionButton action="view.channelRack" variant="outline">
          Go to the channel rack
        </ActionButton>
      </EmptyContent>
    </Empty>
  )
}

export function PlaylistPlaceholder() {
  return (
    <LaterPanel
      icon={Playlist01Icon}
      title="The playlist arrives in phase 2"
      description="This is where patterns will be laid out along a timeline to make a whole song. Until then, build and loop patterns in the channel rack."
    />
  )
}

export function PianoRollPlaceholder() {
  return (
    <LaterPanel
      icon={MusicNote03Icon}
      title="The piano roll arrives in phase 2"
      description="This is where melodies and chords will be drawn note by note. Until then, steps in the channel rack play each sound at its own pitch."
    />
  )
}
