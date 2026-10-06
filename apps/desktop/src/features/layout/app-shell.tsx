import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"
import { ExportDialog } from "@/features/export/export-dialog"
import { CommandPalette } from "@/features/palette/command-palette"
import { SettingsDialog } from "@/features/settings/settings-dialog"
import { TransportBar } from "@/features/transport/transport-bar"
import { resolveTheme, useUiStore } from "@/lib/store/ui"

import { PanelFrame } from "./panel-frame"
import { PANELS, type PanelId } from "./panels"
import { PromptHost } from "./prompt-host"
import { StatusBar } from "./status-bar"
import { TitleBar } from "./title-bar"
import { useAppBoot, useWindowTitle } from "./use-app-boot"
import { Workspace } from "./workspace"

/** Dialogs and toasts every window needs, whatever it shows. */
function Overlays() {
  const theme = useUiStore((state) => resolveTheme(state.theme))
  return (
    <>
      <PromptHost />
      <Toaster theme={theme} position="bottom-right" offset={36} />
    </>
  )
}

/** The main window: menus, transport, docked panels and the status bar. */
export function AppShell() {
  useAppBoot({ guardClose: true })
  useWindowTitle()

  return (
    <TooltipProvider delay={500}>
      <div className="flex h-full flex-col bg-chassis">
        <TitleBar />
        <TransportBar />
        <Workspace />
        <StatusBar />
      </div>
      <CommandPalette />
      <SettingsDialog />
      <ExportDialog />
      <Overlays />
    </TooltipProvider>
  )
}

/**
 * One panel filling a window of its own, opened with `?view=panel&id=mixer`.
 * Panels read everything from the stores, so this is all a detached window
 * needs.
 */
export function PanelWindow({ panel }: { panel: PanelId }) {
  useAppBoot({ guardClose: false })
  const { title, component: Panel } = PANELS[panel]

  return (
    <TooltipProvider delay={500}>
      <div className="h-full">
        <PanelFrame title={title}>
          <Panel />
        </PanelFrame>
      </div>
      <Overlays />
    </TooltipProvider>
  )
}
