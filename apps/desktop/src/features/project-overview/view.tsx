import type { ClipContent } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"

type TargetNames = {
  patterns: Map<number, string>
  samples: Map<number, string>
  automations: Map<number, string>
}

function clipTargetName(content: ClipContent, names: TargetNames): string {
  switch (content.type) {
    case "pattern":
      return `Pattern: ${names.patterns.get(content.pattern) ?? "Missing pattern target"}`
    case "audio":
      return `Sample: ${names.samples.get(content.sample) ?? "Missing sample target"}`
    case "automation":
      return `Automation: ${names.automations.get(content.automation) ?? "Missing automation target"}`
  }
}

/** Document relationships only; no editor selection or command dispatch. */
export function ProjectOverviewView() {
  const project = useProjectStore((state) => state.project)
  const names: TargetNames = {
    patterns: new Map(project.patterns.map(({ id, name }) => [id, name])),
    samples: new Map(project.samples.map(({ id, name }) => [id, name])),
    automations: new Map(project.automations.map(({ id, name }) => [id, name])),
  }
  const tracks = new Map(
    project.playlist.tracks.map(({ id, name }) => [id, name])
  )
  const patterns = project.patterns.map((pattern) => ({
    ...pattern,
    activeChannels: new Set(
      pattern.lanes
        .filter((lane) => lane.notes.length > 0)
        .map((lane) => lane.channel)
    ),
  }))
  const clips = project.playlist.clips.toSorted(
    (a, b) => a.start - b.start || a.id - b.id
  )

  return (
    <div className="flex max-h-[65vh] min-w-0 flex-col gap-4 overflow-y-auto">
      <section aria-label="Channel and pattern relationships">
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-xs">
            <caption className="mb-2 text-left font-medium">
              Channels and patterns
            </caption>
            <thead>
              <tr>
                <th scope="col" className="border p-2 text-left">
                  Channel
                </th>
                {patterns.map((pattern) => (
                  <th
                    key={pattern.id}
                    scope="col"
                    className="min-w-24 border p-2 text-center"
                  >
                    {pattern.name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {project.channels.map((channel) => (
                <tr key={channel.id}>
                  <th scope="row" className="border p-2 text-left font-medium">
                    {channel.name}
                  </th>
                  {patterns.map((pattern) => {
                    const marked = pattern.activeChannels.has(channel.id)
                    return (
                      <td
                        key={pattern.id}
                        aria-label={marked ? "Has notes" : "No notes"}
                        className="border p-2 text-center"
                      >
                        <span aria-hidden="true">{marked ? "●" : "—"}</span>
                      </td>
                    )
                  })}
                </tr>
              ))}
              {project.channels.length === 0 && (
                <tr>
                  <td colSpan={patterns.length + 1} className="p-2">
                    No channels in this project.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
      <section aria-label="Playlist clips" className="flex flex-col gap-2">
        <h2 className="font-medium">Playlist clips</h2>
        <ol
          aria-label="Playlist clips in timeline order"
          className="flex flex-col gap-1"
        >
          {clips.map((clip) => (
            <li
              key={clip.id}
              className="flex flex-wrap gap-x-3 gap-y-1 rounded border p-2"
            >
              <span>{tracks.get(clip.track) ?? "Missing playlist track"}</span>
              <span>{clipTargetName(clip.content, names)}</span>
              <span className="text-muted-foreground">
                Start: {clip.start} ticks
              </span>
            </li>
          ))}
          {clips.length === 0 && <li>No playlist clips in this project.</li>}
        </ol>
      </section>
    </div>
  )
}
