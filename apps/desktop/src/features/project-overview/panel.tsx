import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

import { closeProjectOverview, useProjectOverviewPanel } from "./actions"
import { ProjectOverviewView } from "./view"

export function ProjectOverviewPanel() {
  const open = useProjectOverviewPanel((state) => state.open)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeProjectOverview()
      }}
    >
      <DialogContent className="sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>Project overview</DialogTitle>
          <DialogDescription>
            Channels, patterns and playlist clips in the open Windfall project.
            Read-only; marked cells contain notes.
          </DialogDescription>
        </DialogHeader>
        {open && <ProjectOverviewView />}
      </DialogContent>
    </Dialog>
  )
}
