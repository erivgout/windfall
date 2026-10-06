import { historyJump } from "@/lib/store/project"
import { useHistory } from "@/lib/store/selectors"
import { cn } from "@/lib/utils"

type HistoryRowProps = {
  label: string
  /** How many entries are applied once this row is chosen. */
  cursor: number
  state: "done" | "current" | "undone"
}

function HistoryRow({ label, cursor, state }: HistoryRowProps) {
  return (
    <li>
      <button
        type="button"
        aria-current={state === "current" ? "step" : undefined}
        onClick={() => void historyJump(cursor)}
        className={cn(
          "flex h-6 w-full items-center gap-2 rounded-sm px-2 text-left text-xs outline-none hover:bg-accent focus-visible:bg-accent",
          state === "undone" && "text-muted-foreground/70",
          state === "current" && "font-medium"
        )}
      >
        <span
          aria-hidden
          className={cn(
            "size-1.5 shrink-0 rounded-full",
            state === "current" && "bg-brand",
            state === "done" && "bg-foreground/25",
            state === "undone" && "ring-1 ring-foreground/25 ring-inset"
          )}
        />
        <span className="truncate">{label}</span>
      </button>
    </li>
  )
}

/**
 * The undo history, oldest first. Choosing a row undoes or redoes up to it.
 * Self-contained, so it can sit in a popover or a panel of its own.
 */
export function HistoryList({ className }: { className?: string }) {
  const { entries, cursor } = useHistory()

  const stateAt = (applied: number) =>
    applied === cursor ? "current" : applied < cursor ? "done" : "undone"

  return (
    <ol
      aria-label="Undo history"
      className={cn("flex flex-col gap-px overflow-y-auto", className)}
    >
      <HistoryRow label="Project opened" cursor={0} state={stateAt(0)} />
      {entries.map((entry, index) => (
        <HistoryRow
          key={index}
          label={entry.label}
          cursor={index + 1}
          state={stateAt(index + 1)}
        />
      ))}
    </ol>
  )
}
