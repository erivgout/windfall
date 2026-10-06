import * as React from "react"

import {
  EnvelopeEditor,
  Fader,
  Knob,
  LevelMeter,
  MuteSolo,
  NumberField,
  PanControl,
  PianoKeyboard,
  StepButton,
  StepGrid,
  StepGridGroup,
  ToggleLed,
  Waveform,
  gainToFaderPosition,
  percentUnit,
  type StepGridGroupHandle,
  type WaveformHandle,
} from "@/components/audio"

import { makeDemoPeaks } from "./demo-data"

function Group({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <section className="rounded-lg border bg-card">
      <h2 className="border-b px-3 py-2 text-xs font-semibold">{title}</h2>
      <div className="flex flex-wrap items-end gap-x-6 gap-y-4 p-3">
        {children}
      </div>
    </section>
  )
}

function Cell({
  caption,
  children,
  className,
}: {
  caption: string
  children: React.ReactNode
  className?: string
}) {
  return (
    <figure className={`flex flex-col items-center gap-2 ${className ?? ""}`}>
      <div className="flex min-h-10 items-end justify-center">{children}</div>
      <figcaption className="text-[10px] text-muted-foreground">
        {caption}
      </figcaption>
    </figure>
  )
}

/** A value that the demo cells can change, so every cell is live. */
function useValue(initial: number) {
  const [value, onValueChange] = React.useState(initial)
  return { value, onValueChange }
}

type MeterListener = (left: number, right?: number) => void

function createFeed() {
  const listeners = new Set<MeterListener>()
  return {
    subscribe(listener: MeterListener) {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    emit(left: number, right: number) {
      for (const listener of listeners) {
        listener(left, right)
      }
    },
  }
}

const SIZES = ["sm", "md", "lg"] as const
const STRESS_ROWS = 50
const STRESS_STEPS = 64

function StressRack() {
  const [rows, setRows] = React.useState(() =>
    Array.from({ length: STRESS_ROWS }, (_, row) =>
      Array.from(
        { length: STRESS_STEPS },
        (_, step) => (step * 7 + row * 3) % (4 + (row % 5)) === 0
      )
    )
  )
  const groupRef = React.useRef<StepGridGroupHandle>(null)

  React.useEffect(() => {
    let step = 0
    const timer = setInterval(() => {
      groupRef.current?.setPlayStep(step)
      step = (step + 1) % STRESS_STEPS
    }, 110)
    return () => clearInterval(timer)
  }, [])

  // One stable handler per row keeps a toggle from re-rendering the others.
  const handlers = React.useMemo(
    () =>
      Array.from(
        { length: STRESS_ROWS },
        (_, row) => (step: number, on: boolean) =>
          setRows((current) =>
            current.map((steps, index) =>
              index === row
                ? steps.map((was, at) => (at === step ? on : was))
                : steps
            )
          )
      ),
    []
  )

  return (
    <StepGridGroup
      ref={groupRef}
      data-testid="stress-rack"
      className="w-full gap-0.5"
    >
      {rows.map((steps, row) => (
        <StepGrid
          key={row}
          size="sm"
          steps={steps}
          onToggle={handlers[row]}
          aria-label={`Row ${row + 1}`}
        />
      ))}
    </StepGridGroup>
  )
}

export function KitStates() {
  const knobs = [useValue(0.3), useValue(0.55), useValue(0.8)]
  const pans = [useValue(-0.6), useValue(0), useValue(0.45)]
  const colored = useValue(0.7)
  const stepped = useValue(4)
  const gain = useValue(1)
  const plainFader = useValue(0.6)
  const percentFader = useValue(40)
  const wideFader = useValue(0.7)
  const whole = useValue(12)
  const decimal = useValue(140.5)
  const small = useValue(3)
  const [toggles, setToggles] = React.useState<Record<string, boolean>>({
    button: true,
    dot: true,
    arm: true,
    muted: false,
    solo: true,
  })
  const flip = (name: string) => (pressed: boolean) =>
    setToggles((was) => ({ ...was, [name]: pressed }))
  const [steps, setSteps] = React.useState(() =>
    Array.from({ length: 32 }, (_, step) => step % 3 === 0)
  )
  const [single, setSingle] = React.useState(true)
  const [stress, setStress] = React.useState(false)
  const peaks = React.useMemo(() => makeDemoPeaks(240), [])

  const waveform = React.useRef<WaveformHandle>(null)
  // The meters take their values through `subscribe`, the other way to
  // feed one besides its ref.
  const [feed] = React.useState(createFeed)

  // A slow swell with a hit on top that now and then goes over 0 dB, so the
  // release, the peak hold and the clip light can all be seen.
  React.useEffect(() => {
    let frame = 0
    let burst = 0
    let last = performance.now()
    const start = last
    const tick = (now: number) => {
      const seconds = (now - last) / 1000
      last = now
      const time = (now - start) / 1000
      burst *= Math.exp(-seconds * 6)
      if (Math.floor(time / 1.5) !== Math.floor((time - seconds) / 1.5)) {
        burst = Math.floor(time / 1.5) % 4 === 3 ? 1.25 : 0.7
      }
      const swell = 0.12 + 0.1 * Math.sin(time * 1.7)
      const left = Math.max(burst, swell)
      const right = Math.max(
        burst * 0.8,
        swell * (1 + 0.3 * Math.sin(time * 3.1))
      )
      feed.emit(left, right)
      waveform.current?.setPlayhead((time % 2) / 2)
      frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [feed])

  return (
    <div className="grid grid-cols-2 gap-3" data-testid="states">
      <Group title="Knob">
        {SIZES.map((size, index) => (
          <Cell key={size} caption={`${size}, unipolar`}>
            <Knob
              size={size}
              label="Amount"
              defaultValue={0.5}
              {...knobs[index]}
              {...percentUnit}
            />
          </Cell>
        ))}
        {SIZES.map((size, index) => (
          <Cell key={size} caption={`${size}, bipolar`}>
            <PanControl size={size} label="Pan" {...pans[index]} />
          </Cell>
        ))}
        <Cell caption="value shown">
          <Knob
            label="Drive"
            showValue
            defaultValue={0.5}
            {...knobs[1]}
            {...percentUnit}
          />
        </Cell>
        <Cell caption="own color">
          <Knob
            label="Width"
            color="oklch(0.74 0.13 195)"
            defaultValue={0.5}
            {...colored}
            {...percentUnit}
          />
        </Cell>
        <Cell caption="stepped 0 to 10">
          <Knob
            label="Voices"
            showValue
            min={0}
            max={10}
            step={1}
            defaultValue={4}
            {...stepped}
          />
        </Cell>
        <Cell caption="disabled">
          <Knob label="Amount" disabled value={0.4} />
        </Cell>
      </Group>

      <Group title="Fader">
        <Cell caption="gain, with meter">
          <Fader
            {...gain}
            showValue
            aria-label="Level"
            meter={
              <LevelMeter
                subscribe={feed.subscribe}
                taper={gainToFaderPosition}
              />
            }
          />
        </Cell>
        <Cell caption="no marks">
          <Fader {...gain} ticks={false} aria-label="Level without marks" />
        </Cell>
        <Cell caption="own range and marks">
          <Fader
            {...percentFader}
            min={0}
            max={100}
            step={1}
            defaultValue={50}
            showValue
            format={(value) => `${value}%`}
            ticks={[
              { value: 100, label: "100" },
              { value: 50, label: "50", strong: true },
              { value: 0, label: "0" },
            ]}
            aria-label="Amount"
          />
        </Cell>
        <Cell caption="disabled">
          <Fader value={0.5} disabled aria-label="Disabled level" />
        </Cell>
        <div className="flex flex-col gap-4">
          <Cell caption="horizontal gain">
            <Fader
              orientation="horizontal"
              {...wideFader}
              aria-label="Send level"
            />
          </Cell>
          <Cell caption="horizontal, plain range">
            <Fader
              orientation="horizontal"
              min={0}
              max={1}
              defaultValue={0.5}
              {...plainFader}
              {...percentUnit}
              showValue
              aria-label="Mix"
            />
          </Cell>
        </div>
      </Group>

      <Group title="Level meter">
        <Cell caption="stereo">
          <LevelMeter subscribe={feed.subscribe} />
        </Cell>
        <Cell caption="mono">
          <LevelMeter subscribe={feed.subscribe} channels={1} />
        </Cell>
        <Cell caption="wide, no peak hold">
          <LevelMeter
            subscribe={feed.subscribe}
            peakHoldMs={0}
            className="w-5"
          />
        </Cell>
        <Cell caption="no clip light">
          <LevelMeter subscribe={feed.subscribe} showClip={false} />
        </Cell>
        <div className="flex flex-col gap-4">
          <Cell caption="horizontal stereo">
            <LevelMeter
              subscribe={feed.subscribe}
              orientation="horizontal"
              className="w-48"
            />
          </Cell>
          <Cell caption="horizontal mono">
            <LevelMeter
              subscribe={feed.subscribe}
              orientation="horizontal"
              channels={1}
              className="w-48"
            />
          </Cell>
        </div>
        <p className="max-w-52 self-center text-[11px] text-muted-foreground">
          The clip light stays on after a peak above 0 dB. Click a meter to
          clear it.
        </p>
      </Group>

      <Group title="Toggle and number field">
        {SIZES.map((size) => (
          <Cell key={size} caption={`button ${size}`}>
            <ToggleLed
              size={size}
              pressed={toggles.button}
              onPressedChange={flip("button")}
              aria-label="Example"
            >
              A
            </ToggleLed>
          </Cell>
        ))}
        {SIZES.map((size) => (
          <Cell key={size} caption={`light ${size}`}>
            <ToggleLed
              variant="dot"
              size={size}
              pressed={toggles.dot}
              onPressedChange={flip("dot")}
              color="var(--wf-meter-low)"
              aria-label="Channel on"
            />
          </Cell>
        ))}
        <Cell caption="arm">
          <ToggleLed
            pressed={toggles.arm}
            onPressedChange={flip("arm")}
            color="var(--wf-arm, var(--wf-meter-high))"
            aria-label="Arm for recording"
          >
            R
          </ToggleLed>
        </Cell>
        <Cell caption="mute and solo">
          <MuteSolo
            muted={toggles.muted}
            solo={toggles.solo}
            onMutedChange={flip("muted")}
            onSoloChange={flip("solo")}
          />
        </Cell>
        <Cell caption="disabled">
          <MuteSolo muted solo={false} disabled />
        </Cell>
        <Cell caption="whole numbers">
          <NumberField
            aria-label="Bars"
            min={1}
            max={64}
            defaultValue={8}
            {...whole}
          />
        </Cell>
        <Cell caption="split drag">
          <NumberField
            aria-label="Tempo"
            size="lg"
            min={10}
            max={522}
            step={0.01}
            splitDrag
            defaultValue={140}
            unit="BPM"
            {...decimal}
          />
        </Cell>
        <Cell caption="small">
          <NumberField
            aria-label="Group"
            size="sm"
            min={0}
            max={16}
            {...small}
          />
        </Cell>
        <Cell caption="disabled">
          <NumberField aria-label="Disabled" value={24} disabled />
        </Cell>
      </Group>

      <Group title="Step button and step grid">
        <Cell caption="off">
          <StepButton on={false} aria-label="Off step" />
        </Cell>
        <Cell caption="off, other beat">
          <StepButton on={false} alt aria-label="Off step on the other beat" />
        </Cell>
        <Cell caption="on">
          <StepButton on={single} onToggle={setSingle} aria-label="Step" />
        </Cell>
        <Cell caption="playing">
          <StepButton on={false} playing aria-label="Playing step" />
        </Cell>
        <Cell caption="on, playing">
          <StepButton on playing aria-label="Lit playing step" />
        </Cell>
        <Cell caption="own color">
          <StepButton
            on
            color="oklch(0.74 0.13 195)"
            aria-label="Colored step"
          />
        </Cell>
        <Cell caption="disabled">
          <StepButton on disabled aria-label="Disabled step" />
        </Cell>
        {SIZES.map((size) => (
          <Cell key={size} caption={size}>
            <StepButton on size={size} aria-label={`Step, ${size}`} />
          </Cell>
        ))}
        <Cell caption="32 steps, small" className="w-full items-stretch">
          <StepGrid
            size="sm"
            steps={steps}
            aria-label="Thirty-two steps"
            onToggle={(step, on) =>
              setSteps((was) => was.map((lit, at) => (at === step ? on : lit)))
            }
            className="w-full"
          />
        </Cell>
        <label className="flex w-full items-center gap-2 text-xs">
          <ToggleLed
            data-testid="stress-toggle"
            pressed={stress}
            onPressedChange={setStress}
            aria-label="Show 50 rows of 64 steps"
          >
            50
          </ToggleLed>
          Show 50 rows of 64 steps with a running playhead
        </label>
        {stress ? <StressRack /> : null}
      </Group>

      <Group title="Keyboard, waveform and envelope">
        <Cell caption="piano-roll gutter">
          <PianoKeyboard
            orientation="vertical"
            lowKey={60}
            highKey={83}
            className="h-72 w-16"
          />
        </Cell>
        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <Cell caption="one octave, own color" className="items-stretch">
            <PianoKeyboard
              lowKey={60}
              highKey={72}
              color="oklch(0.74 0.13 195)"
              activeKeys={[64, 67]}
              className="h-14"
            />
          </Cell>
          <Cell caption="disabled" className="items-stretch">
            <PianoKeyboard lowKey={48} highKey={72} disabled className="h-10" />
          </Cell>
          <Cell
            caption="waveform with a playhead, no region"
            className="items-stretch"
          >
            <Waveform ref={waveform} peaks={peaks} normalize className="h-12" />
          </Cell>
          <Cell caption="envelope, disabled" className="items-stretch">
            <EnvelopeEditor
              attackMs={40}
              decayMs={300}
              sustain={0.5}
              releaseMs={900}
              disabled
              className="h-20"
            />
          </Cell>
        </div>
      </Group>
    </div>
  )
}
