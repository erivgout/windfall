import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

import { closePhaseMeter, usePhaseMeterPanel } from "./actions"
import { PhaseMeterView } from "./view"

/** Closing the panel unmounts its realtime subscriber immediately. */
export function PhaseMeterPanel() {
  const open = usePhaseMeterPanel((state) => state.open)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closePhaseMeter()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Phase meter</DialogTitle>
          <DialogDescription>Current stereo correlation.</DialogDescription>
        </DialogHeader>
        {open && <PhaseMeterView />}
      </DialogContent>
    </Dialog>
  )
}
