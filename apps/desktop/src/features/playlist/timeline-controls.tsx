import { useEffect, useState } from "react"
import type { Command, MarkerKind } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Alert, AlertDescription } from "@/components/ui/alert"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldDescription,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { useProjectStore, dispatch } from "@/lib/store/project"
import { MAX_SONG_TICKS } from "@/lib/units"
import { backend } from "@/lib/ipc"
import { useTransportStore } from "@/lib/store/transport"
import {
  getProjectGeneration,
  useProjectGeneration,
} from "@/lib/store/replaced"
import { songTick } from "./ops"
import type { GridMetrics } from "./metrics"
import {
  editTimeline,
  refreshTimelineState,
  useTimelineStore,
  type RulerTool,
  type TimelineEdit,
} from "./timeline-store"
import {
  clearTimelineSelection,
  exportTimelineSelection,
  playTimelineSelection,
} from "./timeline-actions"

function TimelineEditor({
  edit,
  close,
}: {
  edit: TimelineEdit
  close(): void
}) {
  const [tick, setTick] = useState(
    String(edit.item?.tick ?? Math.floor(songTick()))
  )
  const [name, setName] = useState(
    edit.type === "marker" ? (edit.item?.name ?? "Marker") : ""
  )
  const [end, setEnd] = useState(
    String(
      edit.type === "marker" && "end" in edit.kind
        ? edit.kind.end
        : Number(tick) + 3840
    )
  )
  const signature =
    edit.type === "meter"
      ? (edit.item?.signature ??
        useProjectStore.getState().project.settings.timeSignature)
      : null
  const [numerator, setNumerator] = useState(String(signature?.numerator ?? 4))
  const [denominator, setDenominator] = useState(
    String(signature?.denominator ?? 4)
  )
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const save = async () => {
    const at = Number(tick)
    if (
      tick.trim() === "" ||
      !Number.isInteger(at) ||
      at < 0 ||
      at > MAX_SONG_TICKS
    ) {
      setError(`Enter a whole song tick from 0 to ${MAX_SONG_TICKS}.`)
      return
    }
    let command: Command
    if (edit.type === "meter") {
      const signature = {
        numerator: Number(numerator),
        denominator: Number(denominator),
      }
      if (
        !Number.isInteger(signature.numerator) ||
        signature.numerator < 1 ||
        signature.numerator > 16 ||
        ![2, 4, 8, 16].includes(signature.denominator)
      ) {
        setError("Use 1–16 beats with a beat unit of 2, 4, 8 or 16.")
        return
      }
      command = edit.item
        ? { type: "updateMeterChange", id: edit.item.id, tick: at, signature }
        : { type: "addMeterChange", tick: at, signature }
    } else {
      const kind: MarkerKind =
        "end" in edit.kind ? { ...edit.kind, end: Number(end) } : edit.kind
      if (
        !name.trim() ||
        new TextEncoder().encode(name).length > 256 ||
        ("end" in kind &&
          (!Number.isInteger(kind.end) ||
            kind.end <= at ||
            kind.end > MAX_SONG_TICKS))
      ) {
        setError(
          "Give the marker a name of up to 256 bytes and a positive range inside the song bounds."
        )
        return
      }
      command = edit.item
        ? {
            type: "updateTimelineMarker",
            marker: { id: edit.item.id, tick: at, name, kind },
          }
        : { type: "addTimelineMarker", tick: at, name, kind }
    }
    setPending(true)
    const result = await dispatch(command)
    setPending(false)
    if (result) close()
    else
      setError(
        "The timeline edit was refused. Meter ticks must be distinct; navigation marker ranges must not overlap or share endpoints. The error message gives the exact reason."
      )
  }
  const remove = async () => {
    if (!edit.item) return
    setPending(true)
    const result = await dispatch(
      edit.type === "meter"
        ? { type: "removeMeterChange", id: edit.item.id }
        : { type: "removeTimelineMarker", id: edit.item.id }
    )
    setPending(false)
    if (result) close()
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) close()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {edit.item ? "Edit" : "Add"}{" "}
            {edit.type === "meter"
              ? "meter change"
              : `${edit.kind.type} marker`}
          </DialogTitle>
          <DialogDescription>
            {edit.type === "meter"
              ? "The new meter starts a bar here; the preceding bar may be shortened. Clip ticks and durations stay fixed."
              : "Named markers label the view. Loop acts while song looping is enabled; skip jumps forward; pause stops until resumed."}
          </DialogDescription>
        </DialogHeader>
        <form
          noValidate
          onSubmit={(event) => {
            event.preventDefault()
            void save()
          }}
        >
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="timeline-tick">Song tick</FieldLabel>
              <Input
                id="timeline-tick"
                type="number"
                value={tick}
                onChange={(event) => setTick(event.target.value)}
                disabled={pending}
                min={0}
                max={MAX_SONG_TICKS}
              />
            </Field>
            {edit.type === "meter" ? (
              <>
                <Field>
                  <FieldLabel htmlFor="timeline-numerator">
                    Beats per bar
                  </FieldLabel>
                  <Input
                    id="timeline-numerator"
                    type="number"
                    value={numerator}
                    onChange={(event) => setNumerator(event.target.value)}
                    disabled={pending}
                    min={1}
                    max={16}
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="timeline-denominator">
                    Beat unit
                  </FieldLabel>
                  <Input
                    id="timeline-denominator"
                    type="number"
                    value={denominator}
                    onChange={(event) => setDenominator(event.target.value)}
                    disabled={pending}
                  />
                  <FieldDescription>2, 4, 8 or 16</FieldDescription>
                </Field>
              </>
            ) : (
              <>
                <Field>
                  <FieldLabel htmlFor="timeline-name">Marker name</FieldLabel>
                  <Input
                    id="timeline-name"
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                    disabled={pending}
                  />
                </Field>
                {"end" in edit.kind && (
                  <Field>
                    <FieldLabel htmlFor="timeline-end">End tick</FieldLabel>
                    <Input
                      id="timeline-end"
                      type="number"
                      value={end}
                      onChange={(event) => setEnd(event.target.value)}
                      disabled={pending}
                      min={Number(tick) + 1}
                      max={MAX_SONG_TICKS}
                    />
                  </Field>
                )}
              </>
            )}
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <DialogFooter>
              {edit.item && (
                <Button
                  type="button"
                  variant="destructive"
                  disabled={pending}
                  onClick={() => void remove()}
                >
                  Delete
                </Button>
              )}
              <Button
                type="button"
                variant="outline"
                onClick={close}
                disabled={pending}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={pending}>
                Save
              </Button>
            </DialogFooter>
          </FieldGroup>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function TimelineControls({ metrics }: { metrics: GridMetrics }) {
  const timeline = useProjectStore((state) => state.project.playlist.timeline)
  const ready = useProjectStore((state) => state.ready)
  const revision = useProjectStore((state) => state.revision)
  const generation = useProjectGeneration()
  const { tool, selection, active, hydrated, error, edit } = useTimelineStore()
  const choose = editTimeline
  useEffect(() => {
    if (ready) void refreshTimelineState()
  }, [ready, revision, generation])
  useEffect(() => {
    let alive = true
    let reported = 0
    const check = async () => {
      const generation = getProjectGeneration()
      try {
        const state = await backend.timelineState()
        if (!alive || generation !== getProjectGeneration()) return
        if (state.navigationOverflows > reported) {
          reported = state.navigationOverflows
          useTimelineStore.setState({
            error:
              "Playback reached the navigation limit and stopped. Use a longer loop region or move the navigation markers farther apart.",
          })
        }
      } catch {
        /* A later region request reports its IPC error inline. */
      }
    }
    const stop = useTransportStore.subscribe((state, prior) => {
      if (state.mode === "song" && prior.playing && !state.playing) void check()
    })
    return () => {
      alive = false
      stop()
    }
  }, [])
  return (
    <>
      <div className="flex flex-wrap items-center gap-2 border-b px-2 py-1">
        <ToggleGroup
          aria-label="Ruler tool"
          size="sm"
          variant="outline"
          value={[tool]}
          onValueChange={(values) => {
            const next = values[0]
            if (next === "seek" || next === "select" || next === "zoom")
              useTimelineStore.setState({ tool: next satisfies RulerTool })
          }}
        >
          <ToggleGroupItem value="seek">Seek</ToggleGroupItem>
          <ToggleGroupItem value="select">Select time</ToggleGroupItem>
          <ToggleGroupItem value="zoom">Zoom region</ToggleGroupItem>
        </ToggleGroup>
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button variant="outline" size="sm" />}>
            Timeline
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuGroup>
              <DropdownMenuItem onClick={() => choose({ type: "meter" })}>
                Add meter change…
              </DropdownMenuItem>
              {(["named", "loop", "skip", "pause"] as const).map((type) => (
                <DropdownMenuItem
                  key={type}
                  onClick={() =>
                    choose({
                      type: "marker",
                      kind:
                        type === "loop" || type === "skip"
                          ? {
                              type,
                              end:
                                selection?.end ?? Math.floor(songTick()) + 3840,
                            }
                          : { type },
                    })
                  }
                >
                  Add {type} marker…
                </DropdownMenuItem>
              ))}
              <DropdownMenuItem
                disabled={!selection}
                onClick={() => void playTimelineSelection(false)}
              >
                Play selection
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={!selection}
                onClick={() => void playTimelineSelection(true)}
              >
                Loop selection
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={!selection}
                onClick={() => {
                  if (selection) metrics.fitRegion(selection)
                }}
              >
                Zoom to selection
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={!selection}
                onClick={exportTimelineSelection}
              >
                Export selected region…
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={!selection && !active && hydrated}
                onClick={() => void clearTimelineSelection()}
              >
                Clear time selection
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuGroup>
              <DropdownMenuLabel>Meter changes</DropdownMenuLabel>
              {!timeline?.meters.length && (
                <DropdownMenuItem disabled>No meter changes</DropdownMenuItem>
              )}
              {timeline?.meters.map((item) => (
                <DropdownMenuItem
                  key={item.id}
                  onClick={() => choose({ type: "meter", item })}
                >
                  {item.signature.numerator}/{item.signature.denominator} at
                  tick {item.tick}
                </DropdownMenuItem>
              ))}
            </DropdownMenuGroup>
            <DropdownMenuGroup>
              <DropdownMenuLabel>Markers</DropdownMenuLabel>
              {!timeline?.markers.length && (
                <DropdownMenuItem disabled>
                  No timeline markers
                </DropdownMenuItem>
              )}
              {timeline?.markers.map((item) => (
                <DropdownMenuItem
                  key={item.id}
                  onClick={() =>
                    choose({ type: "marker", kind: item.kind, item })
                  }
                >
                  {item.name} ({item.kind.type}, tick {item.tick})
                </DropdownMenuItem>
              ))}
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <span className="text-muted-foreground">
          {selection
            ? `Ticks ${selection.start}–${selection.end}${active ? " · playback region" : ""}`
            : "Drag the ruler to select or zoom; Shift-drag selects time."}
        </span>
      </div>
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      {edit && (
        <TimelineEditor
          key={`${edit.type}-${edit.item?.id ?? (edit.type === "marker" ? edit.kind.type : "new")}`}
          edit={edit}
          close={() => useTimelineStore.setState({ edit: null })}
        />
      )}
    </>
  )
}
