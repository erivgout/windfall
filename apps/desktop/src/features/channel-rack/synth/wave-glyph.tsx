import type { LfoShape, Waveform } from "@/bindings"
import { cn } from "@/lib/utils"

export type WaveShape = Waveform | LfoShape

/** One or two cycles of each shape, drawn in a 16 by 10 box. */
const PATHS: Record<WaveShape, string> = {
  sine: "M1 5C2.6 0 5.4 0 8 5S13.4 10 15 5",
  triangle: "M1 5 4.5 1.5 11.5 8.5 15 5",
  saw: "M1 8.5 8 1.5V8.5L15 1.5V8.5",
  square: "M1 8.5V1.5H8V8.5H15V1.5",
  pulse: "M1 8.5V1.5H3.5V8.5H8V1.5H10.5V8.5H15",
  whiteNoise:
    "M1 5 2.4 2 3.8 8 5.2 3.5 6.6 9 8 1 9.4 6.5 10.8 3 12.2 8 13.6 4 15 6",
  pinkNoise: "M1 5.5 3 3.5 5 6.5 7.5 2 10 8 12.5 4.5 15 6",
  random: "M1 6.5H4V2.5H7V8H10V4.5H13V7H15",
}

type WaveGlyphProps = {
  shape: WaveShape
  className?: string
}

/** A small drawing of a waveform, in the color of the text around it. */
export function WaveGlyph({ shape, className }: WaveGlyphProps) {
  return (
    <svg
      data-slot="wave-glyph"
      data-shape={shape}
      viewBox="0 0 16 10"
      aria-hidden="true"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.25}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("h-2.5 w-4 shrink-0", className)}
    >
      <path d={PATHS[shape]} />
    </svg>
  )
}

export function isWaveShape(value: string): value is WaveShape {
  return value in PATHS
}
