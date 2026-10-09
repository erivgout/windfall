import { backend } from "@/lib/ipc"
import { useProjectStore } from "@/lib/store/project"

import { currentSession } from "./session"
import { buildChannelMusicXml } from "./sheet-music"

export async function exportSheetMusic(): Promise<void> {
  const context = currentSession()?.editor.context
  if (!context) return
  const { project } = useProjectStore.getState()
  const pattern = project.patterns.find(
    (item) => item.id === context.pattern.id
  )
  const channel = project.channels.find((item) => item.id === context.channel)
  if (!pattern || !channel) return
  // Capture the score before waiting on the dialog, even if the open lane changes.
  const xml = buildChannelMusicXml(pattern, channel.id, channel.name)
  await backend.sheetMusicSave(`${pattern.name} - ${channel.name}`, xml)
}
