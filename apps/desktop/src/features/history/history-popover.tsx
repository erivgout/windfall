import { Clock04Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import { Button } from "@/components/ui/button"
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover"
import { useHint } from "@/lib/store/hint"
import { useHistory } from "@/lib/store/selectors"

import { HistoryList } from "./history-list"

/** A button that opens the undo history. */
export function HistoryPopover() {
  const { entries } = useHistory()
  const hint = useHint(
    "History: every edit, in order. Click one to go back to it"
  )

  return (
    <Popover>
      <PopoverTrigger
        render={
          <Button
            variant="ghost"
            size="icon"
            aria-label="Undo history"
            {...hint}
          />
        }
      >
        <HugeiconsIcon icon={Clock04Icon} strokeWidth={2} />
      </PopoverTrigger>
      <PopoverContent align="end" className="w-64 gap-2 p-2">
        <PopoverHeader className="px-1">
          <PopoverTitle className="text-xs">History</PopoverTitle>
          <PopoverDescription>
            {entries.length === 0
              ? "Nothing to undo yet. Edits will be listed here."
              : "Click a step to go back or forward to it."}
          </PopoverDescription>
        </PopoverHeader>
        <HistoryList className="max-h-72" />
      </PopoverContent>
    </Popover>
  )
}
