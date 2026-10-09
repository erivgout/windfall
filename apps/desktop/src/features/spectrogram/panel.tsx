import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

import { closeSpectrogram, useSpectrogramPanel } from "./actions"
import { SpectrogramView } from "./view"

/** Closing the panel immediately unmounts its realtime subscriber. */
export function SpectrogramPanel() {
  const open = useSpectrogramPanel((state) => state.open)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeSpectrogram()
      }}
    >
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Spectrogram</DialogTitle>
          <DialogDescription>
            Short history of engine power. Relative levels without hardware
            calibration.
          </DialogDescription>
        </DialogHeader>
        {open && <SpectrogramView />}
      </DialogContent>
    </Dialog>
  )
}
