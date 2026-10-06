// SPDX-License-Identifier: MIT
import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const stepButtonVariants = cva(
  [
    "relative block min-w-0 shrink-0 rounded-[2px] p-0 outline-none select-none",
    "hover:brightness-110 dark:hover:brightness-125",
    "aria-pressed:bg-[var(--step-on,var(--wf-step-on))] aria-pressed:shadow-[inset_0_1px_0_rgb(255_255_255/0.3),inset_0_-1px_0_rgb(0_0_0/0.15)]",
    // The playhead tints an unlit step and brightens a lit one.
    "data-playing:bg-[color-mix(in_oklch,var(--wf-playhead)_55%,var(--wf-step-off))]",
    "data-playing:aria-pressed:bg-[color-mix(in_oklch,white_45%,var(--step-on,var(--wf-step-on)))]",
    "focus-visible:z-10 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background",
    "disabled:pointer-events-none disabled:opacity-50",
  ],
  {
    variants: {
      // The unlit shade is a plain class, so the lit and playing states,
      // which match on attributes, always win over it.
      alt: {
        false: "bg-(--wf-step-off)",
        true: "bg-(--wf-step-off-alt)",
      },
      size: {
        sm: "h-4 w-2.5",
        md: "h-6 w-3.5",
        lg: "h-8 w-5",
      },
    },
    defaultVariants: { size: "md", alt: false },
  }
)

type StepButtonProps = Omit<React.ComponentProps<"button">, "onToggle"> &
  VariantProps<typeof stepButtonVariants> & {
    on: boolean
    /** Leave unset inside a `StepGrid`, which handles its steps itself. */
    onToggle?: (on: boolean) => void
    /** Use the alternate unlit shade, as every other beat does. */
    alt?: boolean
    playing?: boolean
    /** The lit color as any CSS color. Defaults to `--wf-step-on`. */
    color?: string
  }

/** One step of a step sequencer. */
const StepButton = React.memo(function StepButton({
  className,
  style,
  size = "md",
  on,
  onToggle,
  alt = false,
  playing = false,
  color,
  onClick,
  type = "button",
  ...props
}: StepButtonProps) {
  return (
    <button
      type={type}
      data-slot="step-button"
      data-alt={alt ? "" : undefined}
      data-playing={playing ? "" : undefined}
      aria-pressed={on}
      className={cn(stepButtonVariants({ size, alt }), className)}
      style={
        color
          ? ({ "--step-on": color, ...style } as React.CSSProperties)
          : style
      }
      onClick={
        onToggle || onClick
          ? (event) => {
              onClick?.(event)
              if (!event.defaultPrevented) {
                onToggle?.(!on)
              }
            }
          : undefined
      }
      {...props}
    />
  )
})

export { StepButton, stepButtonVariants }
export type { StepButtonProps }
