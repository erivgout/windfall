import { useId, useState, useSyncExternalStore } from "react"

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

import { ROOT_NAMES } from "./scales"
import {
  applyRiff,
  closeRiffGenerator,
  MAX_RIFF_SEED,
  RIFF_DENSITIES,
  RIFF_SCALES,
  riffCommand,
  riffRequestIsCurrent,
  useRiffGenerator,
  type RiffRequest,
  type RiffSettings,
} from "./riff-generator"

function Choices<T extends string>({
  label,
  value,
  set,
  items,
  disabled,
}: {
  label: string
  value: T
  set(value: T): void
  items: readonly { value: T; label: string }[]
  disabled: boolean
}) {
  return (
    <Field data-disabled={disabled}>
      <FieldLabel>{label}</FieldLabel>
      <ToggleGroup
        aria-label={label}
        value={[value]}
        variant="outline"
        size="sm"
        disabled={disabled}
        onValueChange={(values) => {
          const next = items.find((item) => item.value === values[0])
          if (next) set(next.value)
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

function RiffForm({ request }: { request: RiffRequest }) {
  const id = useId()
  const [root, setRoot] = useState(0)
  const [scale, setScale] = useState<RiffSettings["scale"]>("major")
  const [bars, setBars] = useState("1")
  const [density, setDensity] = useState<RiffSettings["density"]>("medium")
  const [seed, setSeed] = useState("0")
  const [failure, setFailure] = useState<string | null>(null)
  const pending = useRiffGenerator((state) => state.pending)
  useProjectStore((state) => state.revision)
  useSyncExternalStore(
    (listener) => request.session.editor.subscribe(listener),
    () => riffRequestIsCurrent(request)
  )
  const current = riffRequestIsCurrent(request)
  const settings: RiffSettings = {
    root,
    scale,
    bars: bars.trim() ? Number(bars) : NaN,
    density,
    seed: seed.trim() ? Number(seed) : NaN,
  }
  let error: string | null = null
  let count = 0
  try {
    const command = riffCommand(request, settings)
    if (command.type === "addNotes") count = command.notes.length
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Check the riff settings."
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !pending) closeRiffGenerator()
      }}
    >
      <DialogContent
        showCloseButton={!pending}
        className="max-h-[85dvh] overflow-y-auto sm:max-w-lg"
      >
        <form
          className="flex flex-col gap-4"
          onSubmit={async (event) => {
            event.preventDefault()
            if (error || !current || pending) return
            setFailure(null)
            try {
              if (!(await applyRiff(request, settings)))
                setFailure(
                  "The riff could not be added. Close and reopen the generator to try again."
                )
            } catch (cause) {
              setFailure(
                cause instanceof Error
                  ? cause.message
                  : "The riff could not be added."
              )
            }
          }}
        >
          <DialogHeader>
            <DialogTitle>Generate riff</DialogTitle>
            <DialogDescription>
              Append one seeded melody in a chosen scale after this channel's
              notes.
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field data-disabled={pending}>
              <FieldLabel htmlFor={`${id}-root`}>Root</FieldLabel>
              <Select
                value={root}
                items={ROOT_NAMES.map((label, value) => ({ label, value }))}
                disabled={pending}
                onValueChange={(value) => {
                  if (value !== null) setRoot(value)
                }}
              >
                <SelectTrigger id={`${id}-root`}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {ROOT_NAMES.map((label, value) => (
                      <SelectItem key={value} value={value}>
                        {label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>
            <Choices<RiffSettings["scale"]>
              label="Scale"
              value={scale}
              set={setScale}
              items={RIFF_SCALES}
              disabled={pending}
            />
            <Field
              data-invalid={
                !Number.isInteger(settings.bars) ||
                settings.bars < 1 ||
                settings.bars > 4
              }
              data-disabled={pending}
            >
              <FieldLabel htmlFor={`${id}-bars`}>Length (bars)</FieldLabel>
              <Input
                id={`${id}-bars`}
                type="number"
                min={1}
                max={4}
                step={1}
                value={bars}
                disabled={pending}
                aria-invalid={
                  !Number.isInteger(settings.bars) ||
                  settings.bars < 1 ||
                  settings.bars > 4
                }
                onChange={(event) => setBars(event.target.value)}
              />
            </Field>
            <Choices<RiffSettings["density"]>
              label="Density"
              value={density}
              set={setDensity}
              items={RIFF_DENSITIES}
              disabled={pending}
            />
            <Field
              data-invalid={
                !Number.isInteger(settings.seed) ||
                settings.seed < 0 ||
                settings.seed > MAX_RIFF_SEED
              }
              data-disabled={pending}
            >
              <FieldLabel htmlFor={`${id}-seed`}>Seed</FieldLabel>
              <Input
                id={`${id}-seed`}
                type="number"
                min={0}
                max={MAX_RIFF_SEED}
                step={1}
                value={seed}
                disabled={pending}
                aria-invalid={
                  !Number.isInteger(settings.seed) ||
                  settings.seed < 0 ||
                  settings.seed > MAX_RIFF_SEED
                }
                onChange={(event) => setSeed(event.target.value)}
              />
              <FieldDescription>
                Use the same settings and seed to repeat a melody. Snap and draw
                velocity are captured when this dialog opens.
              </FieldDescription>
            </Field>
          </FieldGroup>
          {!error && (
            <p role="status">
              {count} notes · {request.step} ticks per note · one undo step
            </p>
          )}
          {(error || failure || !current) && (
            <Alert variant="destructive">
              <AlertDescription>
                {!current
                  ? "The project or piano-roll channel changed. Close and reopen the generator."
                  : (error ?? failure)}
              </AlertDescription>
            </Alert>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={pending}
              onClick={closeRiffGenerator}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={!!error || !current || pending}>
              {pending ? "Applying…" : "Apply"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function RiffGeneratorDialog() {
  const request = useRiffGenerator((state) => state.request)
  return request ? (
    <RiffForm
      key={`${request.generation}:${request.revision}:${request.context.pattern.id}:${request.context.channel}`}
      request={request}
    />
  ) : null
}
