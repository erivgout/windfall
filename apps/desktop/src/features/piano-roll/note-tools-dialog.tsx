import { useId, useRef, useState, useSyncExternalStore } from "react"

import type { NoteEdge, NoteGroove, NoteTransform } from "@/bindings"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { useProjectStore } from "@/lib/store/project"

import {
  applyNoteTool,
  closeNoteTools,
  NOTE_TOOLS,
  requestIsCurrent,
  useNoteTools,
  type NoteTool,
  type ToolRequest,
} from "./note-tools"
import { currentSession } from "./session"

function NumberField({
  label,
  value,
  set,
  min,
  max,
  step = 1,
  help,
}: {
  label: string
  value: string
  set(value: string): void
  min: number
  max: number
  step?: number
  help?: string
}) {
  const id = useId()
  const n = value.trim() === "" ? NaN : Number(value)
  const valid =
    Number.isFinite(n) &&
    n >= min &&
    n <= max &&
    (step !== 1 || Number.isInteger(n))
  return (
    <Field data-invalid={!valid}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input
        id={id}
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        aria-invalid={!valid}
        onChange={(e) => set(e.target.value)}
      />
      {help && <FieldDescription>{help}</FieldDescription>}
    </Field>
  )
}

function Choices<T extends string>({
  label,
  value,
  set,
  items,
}: {
  label: string
  value: T
  set(value: T): void
  items: { value: T; label: string }[]
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <ToggleGroup
        aria-label={label}
        value={[value]}
        variant="outline"
        size="sm"
        onValueChange={(values) => {
          const selected = items.find((item) => item.value === values[0])
          if (selected) set(selected.value)
        }}
      >
        {items.map((item) => (
          <ToggleGroupItem key={item.value} value={item.value}>
            {item.label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </Field>
  )
}

const GROOVES: { value: NoteGroove; label: string }[] = [
  { value: "straight", label: "Straight" },
  { value: "swing", label: "Swing" },
  { value: "latePairs", label: "Late pairs" },
  { value: "pushFour", label: "Push four" },
]
const number = (value: string) => (value.trim() === "" ? NaN : Number(value))
const within = (n: number, min: number, max: number, integer = false) =>
  Number.isFinite(n) &&
  n >= min &&
  n <= max &&
  (!integer || Number.isInteger(n))

function ToolForm({ request }: { request: ToolRequest }) {
  const [tool, setTool] = useState<NoteTool>(request.tool)
  const [edge, setEdge] = useState<NoteEdge>(request.edge)
  const [groove, setGroove] = useState<NoteGroove>("straight")
  const [grid, setGrid] = useState(String(request.grid))
  const [strength, setStrength] = useState("100")
  const [length, setLength] = useState("50")
  const [velocity, setVelocity] = useState("80")
  const [spacing, setSpacing] = useState("30")
  const [velocityStep, setVelocityStep] = useState("-5")
  const [direction, setDirection] = useState<"up" | "down">("up")
  const [low, setLow] = useState("48")
  const [high, setHigh] = useState("84")
  const [transpose, setTranspose] = useState("0")
  const [range, setRange] = useState<"clamp" | "octaves">("clamp")
  const [pending, setPending] = useState(false)
  const applying = useRef(false)
  const [failure, setFailure] = useState<string | null>(null)
  const session = currentSession()
  useProjectStore((s) => s.revision)
  useSyncExternalStore(
    (notify) => session?.editor.subscribe(notify) ?? (() => {}),
    () => session?.editor.selection
  )
  const current = requestIsCurrent(request)
  const info = NOTE_TOOLS.find((item) => item.value === tool)!

  function transform(): NoteTransform | null {
    const g = number(grid)
    switch (tool) {
      case "quantize":
        return within(g, 1, 245760, true) && within(number(strength), 0, 100)
          ? {
              type: tool,
              grid: g,
              strength: number(strength) / 100,
              edge,
              groove,
            }
          : null
      case "staccato":
        return within(number(length), 1, 100)
          ? { type: tool, factor: number(length) / 100 }
          : null
      case "chop":
        return within(g, 1, 245760, true) ? { type: tool, grid: g } : null
      case "strum":
        return within(number(spacing), 0, 245760, true) &&
          within(number(velocityStep), -100, 100)
          ? {
              type: tool,
              spacing: number(spacing),
              velocityStep: number(velocityStep) / 100,
              descending: direction === "down",
            }
          : null
      case "keyRange":
        return within(number(low), 0, 127, true) &&
          within(number(high), number(low), 127, true) &&
          within(number(transpose), -127, 127, true)
          ? {
              type: tool,
              low: number(low),
              high: number(high),
              transpose: number(transpose),
              octaves: range === "octaves",
            }
          : null
      case "scaleVelocity":
        return within(number(velocity), 0, 400)
          ? { type: tool, factor: number(velocity) / 100 }
          : null
      default:
        return { type: tool }
    }
  }
  const options = transform()
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !applying.current) closeNoteTools()
      }}
    >
      <DialogContent
        showCloseButton={!pending}
        className="max-h-[90vh] overflow-y-auto sm:max-w-md"
        finalFocus={() => session?.focusGrid()}
      >
        <form
          noValidate
          onSubmit={async (event) => {
            event.preventDefault()
            if (!options || applying.current || !requestIsCurrent(request))
              return
            applying.current = true
            setPending(true)
            setFailure(null)
            try {
              const ok = await applyNoteTool(request, options)
              if (ok && useNoteTools.getState().request === request)
                closeNoteTools()
              else
                setFailure(
                  "Could not apply the tool. Check the notification, then reopen with a fresh selection."
                )
            } finally {
              applying.current = false
              setPending(false)
            }
          }}
          className="flex flex-col gap-4"
        >
          <DialogHeader>
            <DialogTitle>Selected-note tools</DialogTitle>
            <DialogDescription>
              {request.notes.length} selected notes. {info.description}
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="note-tool">Tool</FieldLabel>
              <Select
                items={NOTE_TOOLS}
                value={tool}
                onValueChange={(value) => {
                  const next = NOTE_TOOLS.find((item) => item.value === value)
                  if (next) {
                    setTool(next.value)
                    setFailure(null)
                  }
                }}
              >
                <SelectTrigger id="note-tool">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {NOTE_TOOLS.map((item) => (
                      <SelectItem key={item.value} value={item.value}>
                        {item.label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>
            {(tool === "quantize" || tool === "chop") && (
              <NumberField
                label="Grid (ticks)"
                value={grid}
                set={setGrid}
                min={1}
                max={245760}
                help="240 = sixteenth note; 480 = eighth; 960 = quarter. Defaults to the current snap, or 240 when snap is off."
              />
            )}
            {tool === "quantize" && (
              <>
                <Choices<NoteEdge>
                  label="Quantize"
                  value={edge}
                  set={setEdge}
                  items={[
                    { value: "start", label: "Starts" },
                    { value: "end", label: "Ends" },
                  ]}
                />
                <NumberField
                  label="Strength (%)"
                  value={strength}
                  set={setStrength}
                  min={0}
                  max={100}
                  step={0.1}
                  help="100 reaches the grid; 50 moves halfway; 0 leaves timing alone."
                />
                <Choices<NoteGroove>
                  label="Groove"
                  value={groove}
                  set={setGroove}
                  items={GROOVES}
                />
                <FieldDescription>
                  Original rhythms: Swing delays alternate lines by 1/6 of a
                  grid; Late pairs by 1/4; Push four advances every fourth line
                  by 1/6.
                </FieldDescription>
              </>
            )}
            {tool === "staccato" && (
              <NumberField
                label="Length (%)"
                value={length}
                set={setLength}
                min={1}
                max={100}
                step={0.1}
              />
            )}
            {tool === "strum" && (
              <>
                <Choices<"up" | "down">
                  label="Direction"
                  value={direction}
                  set={setDirection}
                  items={[
                    { value: "up", label: "Low to high" },
                    { value: "down", label: "High to low" },
                  ]}
                />
                <NumberField
                  label="Delay per note (ticks)"
                  value={spacing}
                  set={setSpacing}
                  min={0}
                  max={245760}
                />
                <NumberField
                  label="Velocity step (%)"
                  value={velocityStep}
                  set={setVelocityStep}
                  min={-100}
                  max={100}
                  step={0.1}
                  help="Added for each successive note; -5 makes later notes softer."
                />
              </>
            )}
            {tool === "keyRange" && (
              <>
                <NumberField
                  label="Lowest MIDI key"
                  value={low}
                  set={setLow}
                  min={0}
                  max={127}
                  help="60 = C5 in Windfall."
                />
                <NumberField
                  label="Highest MIDI key"
                  value={high}
                  set={setHigh}
                  min={within(number(low), 0, 127) ? number(low) : 0}
                  max={127}
                />
                <NumberField
                  label="Transpose (semitones)"
                  value={transpose}
                  set={setTranspose}
                  min={-127}
                  max={127}
                />
                <Choices<"clamp" | "octaves">
                  label="Range handling"
                  value={range}
                  set={setRange}
                  items={[
                    { value: "clamp", label: "Clamp" },
                    { value: "octaves", label: "Fold octaves" },
                  ]}
                />
              </>
            )}
            {tool === "scaleVelocity" && (
              <NumberField
                label="Velocity scale (%)"
                value={velocity}
                set={setVelocity}
                min={0}
                max={400}
                step={0.1}
                help="100 keeps velocities; 80 makes them 20% softer. Results clamp at full velocity."
              />
            )}
          </FieldGroup>
          {!current && (
            <Alert variant="destructive">
              <AlertDescription>
                The document or selection changed. Close this dialog and reopen
                the tool.
              </AlertDescription>
            </Alert>
          )}
          {failure && (
            <Alert variant="destructive">
              <AlertDescription>{failure}</AlertDescription>
            </Alert>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={pending}
              onClick={closeNoteTools}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={!options || !current || pending}>
              {pending ? "Applying…" : "Apply"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function NoteToolsDialog() {
  const request = useNoteTools((state) => state.request)
  return request ? (
    <ToolForm
      key={`${request.revision}:${request.pattern}:${request.channel}:${request.tool}:${request.edge}`}
      request={request}
    />
  ) : null
}
