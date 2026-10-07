import {
  ArrowDown01Icon,
  CursorRectangleSelection01Icon,
  Eraser01Icon,
  FitToScreenIcon,
  LeftToRightListBulletIcon,
  Magnet01Icon,
  Navigation03Icon,
  PaintBrush01Icon,
  PencilEdit01Icon,
  PlayIcon,
  RepeatIcon,
  VolumeMute01Icon,
} from "@hugeicons/core-free-icons"
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Kbd } from "@/components/ui/kbd"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { ModeSwitch } from "@/features/transport/mode-switch"
import { runAction, useAction } from "@/lib/actions"
import { useHint } from "@/lib/store/hint"
import { useTransportStore } from "@/lib/store/transport"
import { cn } from "@/lib/utils"

import { toolActionId } from "./actions"
import { TOOLS, type Tool } from "./intents"
import { usePlaylistShortcut } from "./keys"
import { SNAP_MODES, type SnapMode } from "./snap"
import { usePlaylistStore } from "./store"

const TOOL_ICONS: Record<Tool, IconSvgElement> = {
  draw: PencilEdit01Icon,
  paint: PaintBrush01Icon,
  select: CursorRectangleSelection01Icon,
  erase: Eraser01Icon,
  mute: VolumeMute01Icon,
}

const TOOL_ABOUT: Record<Tool, string> = {
  draw: "click to place the selected pattern, drag clips to move or resize",
  paint: "drag to lay the selected pattern down back to back",
  select: "drag a box around clips, then move, copy or delete them",
  erase: "click or drag across clips to delete them",
  mute: "click or drag across clips to mute or unmute them",
}

type IconActionProps = {
  /** Id of the registry action the button runs. */
  action: string
  icon: IconSvgElement
  /** For an action that is on or off. */
  pressed?: boolean
  /** Added to the status bar hint after the title. */
  about?: string
  className?: string
}

/**
 * A toolbar button for one of the playlist's actions. It shows the shortcut
 * the action has inside the playlist, which the app-wide keymap may have
 * given to another panel.
 */
function IconAction({
  action: id,
  icon,
  pressed,
  about,
  className,
}: IconActionProps) {
  const action = useAction(id)
  const shortcut = usePlaylistShortcut(id)
  const title = action?.title.replace(/…$/, "") ?? id
  const hint = useHint(
    [title, shortcut ? ` (${shortcut})` : "", about ? `: ${about}` : ""].join(
      ""
    )
  )

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            variant={pressed ? "secondary" : "ghost"}
            size="icon-sm"
            aria-label={title}
            aria-pressed={pressed}
            className={cn(
              "text-muted-foreground aria-pressed:text-foreground",
              className
            )}
            onClick={() => void runAction(id)}
            {...hint}
          />
        }
      >
        <HugeiconsIcon icon={icon} strokeWidth={2} className="size-3.5" />
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {title}
        {shortcut && <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  )
}

function Tools() {
  const tool = usePlaylistStore((state) => state.tool)
  return (
    <div
      role="group"
      aria-label="Tools"
      className="flex items-center rounded-md bg-(--wf-step-off)/60 p-px"
    >
      {TOOLS.map((item) => (
        <IconAction
          key={item}
          action={toolActionId(item)}
          icon={TOOL_ICONS[item]}
          pressed={item === tool}
          about={TOOL_ABOUT[item]}
          className="aria-pressed:bg-background aria-pressed:shadow-xs"
        />
      ))}
    </div>
  )
}

function SnapMenu() {
  const snap = usePlaylistStore((state) => state.snap)
  const setSnap = usePlaylistStore((state) => state.setSnap)
  const current = SNAP_MODES.find((item) => item.mode === snap)
  const hint = useHint(
    "Snap: what clips line up with when placed, moved and resized. Hold Alt to ignore it for one drag"
  )

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="outline"
            size="sm"
            aria-label={`Snap: ${current?.label ?? snap}`}
            className="gap-1.5 px-1.5"
            {...hint}
          />
        }
      >
        <HugeiconsIcon
          icon={Magnet01Icon}
          strokeWidth={2}
          className={snap === "none" ? "text-muted-foreground" : "text-brand"}
        />
        <span className="w-7 text-left">{current?.label}</span>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          strokeWidth={2}
          className="text-muted-foreground"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56">
        <DropdownMenuGroup>
          <DropdownMenuLabel>Snap</DropdownMenuLabel>
          <DropdownMenuRadioGroup
            value={snap}
            onValueChange={(mode: SnapMode) => setSnap(mode)}
          >
            {SNAP_MODES.map((item) => (
              <DropdownMenuRadioItem
                key={item.mode}
                value={item.mode}
                closeOnClick
              >
                <span>{item.label}</span>
                <span className="ml-auto pl-3 text-muted-foreground">
                  {item.about}
                </span>
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

/**
 * Says what Play plays and changes it. The playhead only runs along the
 * playlist in song mode, so while the transport loops a pattern a button
 * offers to play the song instead.
 */
function SongControls() {
  const mode = useTransportStore((state) => state.mode)
  const playing = useTransportStore((state) => state.playing)
  const loop = useTransportStore((state) => state.loopSong)
  const follow = usePlaylistStore((state) => state.follow)
  const playHint = useHint(
    "Play the song: switches the transport from the pattern to the playlist and starts it"
  )

  return (
    <div className="ml-auto flex items-center gap-1.5 pl-2">
      {mode === "pattern" && (
        <Button
          size="sm"
          className="gap-1 bg-brand text-brand-foreground hover:bg-brand/85"
          onClick={() => void runAction("playlist.playSong")}
          {...playHint}
        >
          <HugeiconsIcon icon={PlayIcon} strokeWidth={2} />
          Play song
        </Button>
      )}
      <span
        className={cn(
          "text-muted-foreground",
          mode === "song" && playing && "text-brand"
        )}
      >
        {mode === "song"
          ? "Playing from the playlist"
          : "Play loops the pattern"}
      </span>
      <ModeSwitch />
      <IconAction
        action="playlist.loopSong"
        icon={RepeatIcon}
        pressed={loop}
        about="at the end of the last clip, start again from the top"
      />
      <IconAction
        action="playlist.follow"
        icon={Navigation03Icon}
        pressed={follow}
        about="scroll along while the song plays"
      />
    </div>
  )
}

/** The strip above the timeline: pattern list, tools, snap, zoom, song mode. */
export function PlaylistToolbar() {
  const pickerOpen = usePlaylistStore((state) => state.pickerOpen)

  return (
    <div
      role="toolbar"
      aria-label="Playlist"
      className="flex h-9 shrink-0 items-center gap-2 overflow-x-auto overflow-y-hidden border-b bg-chassis/40 px-1.5 whitespace-nowrap"
    >
      <IconAction
        action="playlist.patterns"
        icon={LeftToRightListBulletIcon}
        pressed={pickerOpen}
        about="show or hide the patterns to place"
      />
      <Tools />
      <SnapMenu />
      <IconAction
        action="playlist.zoomToFit"
        icon={FitToScreenIcon}
        about="show the whole song"
      />
      <SongControls />
    </div>
  )
}
