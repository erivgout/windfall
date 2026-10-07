// SPDX-License-Identifier: MIT
import * as React from "react"

import { cn } from "@/lib/utils"

type ValueInputProps = Omit<
  React.ComponentProps<"input">,
  "defaultValue" | "value" | "onChange"
> & {
  initialText: string
  /** Select the text so typing replaces it. */
  selectAll?: boolean
  onCommit: (text: string) => void
  /**
   * Called when the entry closes without a value: `"escape"` for the key,
   * `"blur"` when the field lost the focus.
   */
  onCancel: (reason: "escape" | "blur") => void
}

/**
 * The inline text entry the value controls open on Enter, on a typed digit
 * or on double-click. Enter and Tab commit. Escape cancels, and so does
 * leaving the field any other way: a value is only ever set on purpose, so
 * an entry opened by a stray key and then clicked away from changes nothing.
 */
function ValueInput({
  initialText,
  selectAll = true,
  onCommit,
  onCancel,
  className,
  ...props
}: ValueInputProps) {
  const ref = React.useRef<HTMLInputElement>(null)
  const settled = React.useRef(false)

  React.useLayoutEffect(() => {
    const input = ref.current
    if (!input) {
      return
    }
    input.focus({ preventScroll: true })
    if (selectAll) {
      input.select()
    } else {
      input.setSelectionRange(input.value.length, input.value.length)
    }
  }, [selectAll])

  function settle(how: "commit" | "escape" | "blur", text: string) {
    // Closing the field blurs it, which must not settle it a second time.
    if (settled.current) {
      return
    }
    settled.current = true
    // Committing the untouched readout would round the value to what the
    // readout shows.
    const untouched = selectAll && text === initialText
    if (how === "commit" && !untouched) {
      onCommit(text)
    } else {
      onCancel(how === "blur" ? "blur" : "escape")
    }
  }

  return (
    <input
      ref={ref}
      data-slot="value-input"
      type="text"
      inputMode="decimal"
      autoComplete="off"
      spellCheck={false}
      defaultValue={initialText}
      className={cn(
        // select-text: the controls around it switch text selection off, which
        // would stop some browsers from editing the field.
        "h-5 min-w-0 rounded-sm border border-ring bg-background px-1 text-center text-[11px] leading-none text-foreground tabular-nums ring-2 ring-ring/30 outline-none select-text",
        className
      )}
      onKeyDown={(event) => {
        // Keys typed here are text, not app shortcuts or slider steps.
        event.stopPropagation()
        if (event.key === "Enter") {
          event.preventDefault()
          settle("commit", event.currentTarget.value)
        } else if (event.key === "Tab") {
          // Not prevented: the focus moves on as Tab always does.
          settle("commit", event.currentTarget.value)
        } else if (event.key === "Escape") {
          event.preventDefault()
          settle("escape", "")
        }
      }}
      onBlur={() => settle("blur", "")}
      onPointerDown={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      {...props}
    />
  )
}

export { ValueInput }
export type { ValueInputProps }
