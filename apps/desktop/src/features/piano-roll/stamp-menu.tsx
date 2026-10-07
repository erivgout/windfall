import { useRef, useSyncExternalStore } from "react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { getProjectGeneration } from "@/lib/store/replaced"

import { useSession } from "./context"
import type { EditorContext } from "./editor"
import { CHORD_STAMPS, SCALE_STAMPS, type Stamp } from "./stamps"

export function StampMenu() {
  const session = useSession()
  const { editor } = session
  const pending = useRef<{
    stamp: Stamp
    context: EditorContext
    generation: number
  } | null>(null)
  const state = useSyncExternalStore(
    (listener) => editor.subscribe(listener),
    () => editor.stampState,
    () => null
  )
  const choose = (stamp: Stamp) => {
    const context = editor.context
    if (context)
      pending.current = { stamp, context, generation: getProjectGeneration() }
  }

  return (
    <>
      <DropdownMenu
        onOpenChangeComplete={(open) => {
          if (open) return
          const chosen = pending.current
          pending.current = null
          if (!chosen || chosen.generation !== getProjectGeneration()) return
          const current = editor.context
          if (
            current?.channel !== chosen.context.channel ||
            current.pattern.id !== chosen.context.pattern.id ||
            current.notes !== chosen.context.notes ||
            current.pattern.lengthSteps !==
              chosen.context.pattern.lengthSteps ||
            current.pattern.signature.numerator !==
              chosen.context.pattern.signature.numerator ||
            current.pattern.signature.denominator !==
              chosen.context.pattern.signature.denominator
          )
            return
          // Closing menus manage focus until their exit transition completes.
          editor.armStamp(chosen.stamp)
          session.focusGrid()
        }}
      >
        <DropdownMenuTrigger
          render={
            <Button
              variant={state ? "secondary" : "outline"}
              size="sm"
              aria-label="Choose chord or scale stamp"
            />
          }
        >
          Stamp
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-80">
          <DropdownMenuGroup>
            <DropdownMenuLabel>
              Choose a pattern, then click its root in the grid
            </DropdownMenuLabel>
            <DropdownMenuSub>
              <DropdownMenuSubTrigger>Chords</DropdownMenuSubTrigger>
              <DropdownMenuSubContent className="w-64">
                <DropdownMenuGroup>
                  <DropdownMenuLabel>Simultaneous notes</DropdownMenuLabel>
                  {CHORD_STAMPS.map((stamp) => (
                    <DropdownMenuItem
                      key={stamp.id}
                      onClick={() => choose(stamp)}
                    >
                      {stamp.label}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuGroup>
              </DropdownMenuSubContent>
            </DropdownMenuSub>
            {(["ascending", "descending"] as const).map((direction) => (
              <DropdownMenuSub key={direction}>
                <DropdownMenuSubTrigger>
                  {direction === "ascending"
                    ? "Ascending scales"
                    : "Descending scales"}
                </DropdownMenuSubTrigger>
                <DropdownMenuSubContent className="w-72">
                  <DropdownMenuGroup>
                    <DropdownMenuLabel>
                      One note length per scale step
                    </DropdownMenuLabel>
                    {SCALE_STAMPS.filter(
                      (stamp) => stamp.layout === direction
                    ).map((stamp) => (
                      <DropdownMenuItem
                        key={stamp.id}
                        onClick={() => choose(stamp)}
                      >
                        {stamp.label}
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuGroup>
                </DropdownMenuSubContent>
              </DropdownMenuSub>
            ))}
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>
      {state && (
        <>
          <Button
            variant="ghost"
            size="sm"
            aria-label="Cancel stamp placement"
            onClick={() => {
              editor.cancel()
              session.focusGrid()
            }}
          >
            Cancel
          </Button>
          <span
            role="status"
            aria-label="Stamp placement"
            className="max-w-80 shrink-0 truncate text-muted-foreground"
            title={state.error ?? undefined}
          >
            {state.error ??
              `${state.stamp.label}: click to place · Esc cancels`}
          </span>
        </>
      )}
    </>
  )
}
