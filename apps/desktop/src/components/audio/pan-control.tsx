// SPDX-License-Identifier: MIT
import { Knob, type KnobProps } from "./knob"
import { formatPan, parsePan } from "./units"

type PanControlProps = Omit<KnobProps, "min" | "max" | "bipolar" | "center">

/**
 * A pan knob: -1 (left) to 1 (right) with a center detent, read out as
 * "L50", "C" and "R50". Double-click returns it to the center.
 */
function PanControl({
  defaultValue = 0,
  format = formatPan,
  parse = parsePan,
  "aria-label": ariaLabel,
  label,
  ...props
}: PanControlProps) {
  return (
    <Knob
      data-slot="pan-control"
      min={-1}
      max={1}
      bipolar
      defaultValue={defaultValue}
      format={format}
      parse={parse}
      label={label}
      aria-label={ariaLabel ?? (label ? undefined : "Pan")}
      {...props}
    />
  )
}

export { PanControl }
export type { PanControlProps }
