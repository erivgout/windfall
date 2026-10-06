import {
  Popover,
  PopoverContent,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import type { TimeSignature } from "@/bindings"
import { useHint } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { useSettings } from "@/lib/store/selectors"

const NUMERATORS = Array.from({ length: 16 }, (_, index) => index + 1)
const DENOMINATORS = [2, 4, 8, 16]

function NumberSelect({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: number
  options: number[]
  onChange(value: number): void
}) {
  return (
    <Select
      value={value}
      onValueChange={(next: number | null) => {
        if (next !== null) onChange(next)
      }}
    >
      <SelectTrigger aria-label={label} className="w-16">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {options.map((option) => (
            <SelectItem key={option} value={option}>
              {option}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}

/** The time signature, shown in the display and edited in a small popover. */
export function TimeSignatureField() {
  const { timeSignature } = useSettings()
  const hint = useHint(
    "Time signature: beats per bar and the note that is one beat"
  )

  function change(next: TimeSignature) {
    void dispatch({ type: "updateSettings", patch: { timeSignature: next } })
  }

  return (
    <Popover>
      <PopoverTrigger
        aria-label={`Time signature: ${timeSignature.numerator} over ${timeSignature.denominator}`}
        className="flex h-6 items-center rounded-sm px-1 font-readout text-[0.8125rem] font-medium text-display-foreground outline-none hover:bg-display-foreground/10 focus-visible:ring-1 focus-visible:ring-brand aria-expanded:bg-display-foreground/10"
        {...hint}
      >
        {timeSignature.numerator}/{timeSignature.denominator}
      </PopoverTrigger>
      <PopoverContent className="w-auto gap-3" align="center">
        <PopoverHeader>
          <PopoverTitle className="text-xs">Time signature</PopoverTitle>
        </PopoverHeader>
        <div className="flex items-center gap-2">
          <NumberSelect
            label="Beats per bar"
            value={timeSignature.numerator}
            options={NUMERATORS}
            onChange={(numerator) => change({ ...timeSignature, numerator })}
          />
          <span className="text-muted-foreground" aria-hidden>
            /
          </span>
          <NumberSelect
            label="Beat unit"
            value={timeSignature.denominator}
            options={DENOMINATORS}
            onChange={(denominator) =>
              change({ ...timeSignature, denominator })
            }
          />
        </div>
      </PopoverContent>
    </Popover>
  )
}
