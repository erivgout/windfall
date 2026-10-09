import { useId, type ReactNode } from "react"

import { Badge } from "@/components/ui/badge"
import { Empty, EmptyHeader, EmptyTitle } from "@/components/ui/empty"
import { effectDescriptor } from "@/features/params/descriptors"
import { useProjectStore } from "@/lib/store/project"

function ProjectGroup({
  title,
  children,
}: {
  title: string
  children: ReactNode
}) {
  const heading = useId()
  return (
    <section aria-labelledby={heading} className="flex flex-col gap-1">
      <h2 id={heading} className="text-xs font-semibold text-muted-foreground">
        {title}
      </h2>
      {children}
    </section>
  )
}

function NamedGroup({
  title,
  entries,
  always = false,
}: {
  title: string
  entries: readonly { name: string }[]
  always?: boolean
}) {
  if (!always && entries.length === 0) return null
  return (
    <ProjectGroup title={title}>
      {entries.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyTitle>No patterns yet</EmptyTitle>
          </EmptyHeader>
        </Empty>
      ) : (
        <ul className="flex flex-col gap-1">
          {entries.map((entry, index) => (
            <li key={index} className="rounded-sm px-2 py-1 break-words">
              {entry.name}
            </li>
          ))}
        </ul>
      )}
    </ProjectGroup>
  )
}

/** A live inventory of the open document, with no editor or command actions. */
export function ProjectTab() {
  const project = useProjectStore((state) => state.project)
  const history = useProjectStore((state) => state.history)
  const effectTracks = project.mixer.tracks.filter(
    (track) => track.effects.length > 0
  )

  return (
    <div data-slot="project-browser" className="flex flex-col gap-4 p-2">
      <p className="text-xs text-muted-foreground">Open project · Read-only</p>
      <NamedGroup title="Patterns" entries={project.patterns} always />
      <NamedGroup title="Channels" entries={project.channels} />
      <NamedGroup title="Samples" entries={project.samples} />
      {effectTracks.length > 0 && (
        <ProjectGroup title="Mixer effects">
          {effectTracks.map((track) => (
            <section key={track.id} aria-label={track.name} className="px-2">
              <h3 className="text-xs font-medium break-words">{track.name}</h3>
              <ul className="flex flex-col gap-1">
                {track.effects.map((slot) => {
                  const plugin = project.plugins?.find(
                    (binding) =>
                      binding.target.type === "effect" &&
                      binding.target.effect === slot.id
                  )
                  return (
                    <li
                      key={slot.id}
                      className="flex items-center gap-2 py-1 pl-2"
                    >
                      <span className="min-w-0 break-words">
                        {plugin?.name ??
                          effectDescriptor(slot.params.type).name}
                      </span>
                      {!slot.enabled && (
                        <Badge variant="outline">Bypassed</Badge>
                      )}
                    </li>
                  )
                })}
              </ul>
            </section>
          ))}
        </ProjectGroup>
      )}
      <NamedGroup title="Automations" entries={project.automations} />
      <NamedGroup title="Plugins" entries={project.plugins ?? []} />
      <NamedGroup
        title="Retained plugin states"
        entries={(project.retainedPlugins ?? []).map((plugin) => ({
          name: plugin.name ?? plugin.internalName,
        }))}
      />
      {history.entries.length > 0 && (
        <ProjectGroup title="Edit history">
          <ol className="flex flex-col gap-1">
            {history.entries.map((entry, index) => {
              const undone = index >= history.cursor
              return (
                <li
                  key={index}
                  data-undone={undone}
                  className="flex items-center gap-2 rounded-sm px-2 py-1 data-[undone=true]:text-muted-foreground"
                >
                  <span className="min-w-0 break-words">{entry.label}</span>
                  <Badge variant={undone ? "outline" : "secondary"}>
                    {undone ? "Undone" : "Applied"}
                  </Badge>
                </li>
              )
            })}
          </ol>
        </ProjectGroup>
      )}
    </div>
  )
}
