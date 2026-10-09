import { ActionButton } from "@/components/action-button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

import {
  closeGettingStarted,
  useGettingStartedStore,
} from "./getting-started-store"

const STEPS = [
  {
    title: "Add a channel",
    sentence: "Add an instrument channel.",
    action: "channel.add",
  },
  {
    title: "Open the piano roll",
    sentence: "Draw notes on that channel.",
    action: "view.pianoRoll",
  },
  {
    title: "Open the playlist",
    sentence: "Place the pattern on the timeline.",
    action: "view.playlist",
  },
  {
    title: "Play",
    sentence: "Start playback from the transport.",
    action: "transport.play",
  },
  {
    title: "Export",
    sentence: "Write the mix to an audio file.",
    action: "file.export",
  },
]

export function GettingStartedDialog() {
  const open = useGettingStartedStore((state) => state.open)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeGettingStarted()
      }}
    >
      <DialogContent className="max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Getting started</DialogTitle>
          <DialogDescription>
            Use these actions to make your first track.
          </DialogDescription>
        </DialogHeader>
        <ol className="flex list-decimal flex-col gap-4 pl-5">
          {STEPS.map((step) => (
            <li key={step.action}>
              <div className="flex flex-col items-start gap-1">
                <p className="text-muted-foreground">{step.sentence}</p>
                <ActionButton action={step.action} variant="outline" size="sm">
                  {step.title}
                </ActionButton>
              </div>
            </li>
          ))}
        </ol>
      </DialogContent>
    </Dialog>
  )
}
