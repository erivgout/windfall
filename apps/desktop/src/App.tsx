import { AppShell, PanelWindow } from "@/features/layout/app-shell"
import { isPanelId } from "@/features/layout/panels"

/**
 * The main window, or with `?view=panel&id=mixer` a single panel filling the
 * window, which is how a detached panel will be shown.
 */
export function App() {
  const query = new URLSearchParams(window.location.search)
  const panel = query.get("id")
  if (query.get("view") === "panel" && isPanelId(panel)) {
    return <PanelWindow panel={panel} />
  }
  return <AppShell />
}

export default App
