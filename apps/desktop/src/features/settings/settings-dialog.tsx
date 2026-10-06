import { useEffect, type ReactNode } from "react"

import type { AudioSettings } from "@/bindings"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { KEYMAP_PRESETS } from "@/lib/actions/keymap"
import {
  configureEngine,
  loadAudioDevices,
  useEngineStore,
} from "@/lib/store/engine"
import { useUiStore, type KeymapPreset, type Theme } from "@/lib/store/ui"
import { formatSampleRate } from "@/lib/time"
import { cn } from "@/lib/utils"

const BUFFER_SIZES = [32, 64, 128, 256, 512, 1024, 2048, 4096]

type Option<T> = { value: T; label: string }

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[7.5rem_1fr] items-center gap-3">
      <span className="text-muted-foreground">{label}</span>
      {children}
    </div>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-xs font-medium">{title}</h3>
      {children}
    </section>
  )
}

function Choice<T extends string | number>({
  label,
  value,
  options,
  disabled,
  onChange,
}: {
  label: string
  value: T | null
  options: Option<T>[]
  disabled?: boolean
  onChange(value: T): void
}) {
  return (
    <Select
      items={options}
      value={value}
      disabled={disabled}
      onValueChange={(next: T | null) => {
        if (next !== null) onChange(next)
      }}
    >
      <SelectTrigger aria-label={label} className="w-full">
        <SelectValue placeholder="Not available" />
      </SelectTrigger>
      <SelectContent alignItemWithTrigger={false}>
        <SelectGroup>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}

function AudioSection() {
  const status = useEngineStore((state) => state.status)
  const hosts = useEngineStore((state) => state.hosts)

  useEffect(() => {
    void loadAudioDevices()
  }, [])

  const host = hosts.find((item) => item.name === status?.host)
  const device = host?.devices.find((item) => item.name === status?.device)
  const requested: AudioSettings = {
    host: status?.host,
    device: status?.device ?? undefined,
    sampleRate: status?.sampleRate,
    bufferFrames: status?.bufferFrames,
  }
  const apply = (change: AudioSettings) =>
    void configureEngine({ ...requested, ...change })

  const bufferSizes = BUFFER_SIZES.filter(
    (size) =>
      size >= (device?.minBufferFrames ?? 0) &&
      size <= (device?.maxBufferFrames ?? Infinity)
  )
  const loading = hosts.length === 0

  return (
    <Section title="Audio output">
      <Row label="Driver">
        <Choice
          label="Audio driver"
          value={status?.host ?? null}
          disabled={loading}
          options={hosts.map((item) => ({
            value: item.name,
            label: item.name,
          }))}
          // Another driver has other devices, so let it pick its default.
          onChange={(name) => apply({ host: name, device: undefined })}
        />
      </Row>
      <Row label="Device">
        <Choice
          label="Output device"
          value={status?.device ?? null}
          disabled={loading}
          options={(host?.devices ?? []).map((item) => ({
            value: item.name,
            label: item.name,
          }))}
          onChange={(name) => apply({ device: name })}
        />
      </Row>
      <Row label="Sample rate">
        <Choice
          label="Sample rate"
          value={status?.sampleRate ?? null}
          disabled={loading || !device}
          options={(device?.sampleRates ?? []).map((rate) => ({
            value: rate,
            label: formatSampleRate(rate),
          }))}
          onChange={(sampleRate) => apply({ sampleRate })}
        />
      </Row>
      <Row label="Buffer size">
        <Choice
          label="Buffer size"
          value={status?.bufferFrames ?? null}
          disabled={loading || !device}
          options={bufferSizes.map((size) => ({
            value: size,
            label: `${size} samples`,
          }))}
          onChange={(bufferFrames) => apply({ bufferFrames })}
        />
      </Row>
      <p
        role="status"
        className={cn(
          "rounded-md bg-muted px-2.5 py-1.5 text-muted-foreground",
          status && !status.running && "bg-destructive/10 text-destructive"
        )}
      >
        {!status && "Waiting for the audio engine."}
        {status?.running &&
          `Running with ${status.latencyMs.toFixed(1)} ms of output latency. A smaller buffer lowers latency and raises CPU load.`}
        {status &&
          !status.running &&
          (status.error ?? "The audio output is not running.")}
      </p>
    </Section>
  )
}

const THEMES: Option<Theme>[] = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "Match the system" },
]

function AppearanceSection() {
  const theme = useUiStore((state) => state.theme)
  const setTheme = useUiStore((state) => state.setTheme)
  return (
    <Section title="Appearance">
      <Row label="Theme">
        <Choice
          label="Theme"
          value={theme}
          options={THEMES}
          onChange={setTheme}
        />
      </Row>
    </Section>
  )
}

function KeyboardSection() {
  const keymap = useUiStore((state) => state.keymap)
  const setKeymap = useUiStore((state) => state.setKeymap)
  const preset = KEYMAP_PRESETS.find((item) => item.id === keymap)
  return (
    <Section title="Keyboard">
      <Row label="Shortcuts">
        <Choice<KeymapPreset>
          label="Keyboard shortcuts"
          value={keymap}
          options={KEYMAP_PRESETS.map((item) => ({
            value: item.id,
            label: item.name,
          }))}
          onChange={setKeymap}
        />
      </Row>
      <p className="text-muted-foreground">
        {preset?.about} The command palette lists every shortcut.
      </p>
    </Section>
  )
}

/** Audio device, theme and keyboard shortcuts. */
export function SettingsDialog() {
  const open = useUiStore((state) => state.dialog === "settings")
  const closeDialog = useUiStore((state) => state.closeDialog)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeDialog()
      }}
    >
      <DialogContent className="gap-5 sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>
            Changes take effect right away and are kept for next time.
          </DialogDescription>
        </DialogHeader>
        <AudioSection />
        <AppearanceSection />
        <KeyboardSection />
      </DialogContent>
    </Dialog>
  )
}
