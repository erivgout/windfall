import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

import { closeSpectrum, useSpectrumPanel } from "./actions"
import { SpectrumView } from "./view"

/** Closing the panel unmounts its realtime subscriber immediately. */
export function SpectrumPanel() {
  const open = useSpectrumPanel((state) => state.open)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeSpectrum()
      }}
    >
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Spectrum</DialogTitle>
          <DialogDescription>
            Current engine spectrum. Relative power, without history or hardware
            calibration.
          </DialogDescription>
        </DialogHeader>
        {open && <SpectrumView />}
      </DialogContent>
    </Dialog>
  )
}
