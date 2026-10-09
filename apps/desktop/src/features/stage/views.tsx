import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

import { closeStageView, useStageView } from "./actions"
import { LargeClock } from "./large-clock"
import { LargeMasterMeter } from "./large-master-meter"

/** Only the open display subscribes to realtime frames. */
export function StageViews() {
  const view = useStageView((state) => state.view)

  return (
    <Dialog
      open={view !== null}
      onOpenChange={(open) => {
        if (!open) closeStageView()
      }}
    >
      <DialogContent className="max-h-[calc(100%-2rem)] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {view === "meter" ? "Large master meter" : "Large clock"}
          </DialogTitle>
          <DialogDescription>
            {view === "meter"
              ? "Master peak levels. Click the meter to clear its clip light."
              : "Transport position in bars, beats and steps, and minutes and seconds."}
          </DialogDescription>
        </DialogHeader>
        {view === "clock" && <LargeClock />}
        {view === "meter" && <LargeMasterMeter />}
      </DialogContent>
    </Dialog>
  )
}
