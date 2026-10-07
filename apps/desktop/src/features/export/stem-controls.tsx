import type { ExportStems } from "@/bindings"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { useProjectStore } from "@/lib/store/project"
import { MASTER_TRACK } from "@/lib/units"

import { Choice } from "./choice"

const DEFAULT_STEMS: ExportStems = {
  mode: "trackOutputs",
  includeMix: true,
  numbered: true,
  folder: true,
}

export function StemControls({
  value,
  disabled,
  onChange,
}: {
  value: ExportStems | undefined
  disabled: boolean
  onChange(value: ExportStems | undefined): void
}) {
  const mixer = useProjectStore((state) => state.project.mixer)
  const tracks = mixer.tracks.filter((track) => track.id !== MASTER_TRACK)
  const set = <K extends keyof ExportStems>(key: K, next: ExportStems[K]) => {
    if (value) onChange({ ...value, [key]: next })
  }
  return (
    <FieldSet disabled={disabled}>
      <FieldLegend variant="label">Files to export</FieldLegend>
      <FieldGroup className="gap-3">
        <Field orientation="horizontal" data-disabled={disabled || undefined}>
          <Checkbox
            id="export-stems"
            checked={value !== undefined}
            disabled={disabled}
            onCheckedChange={(checked) =>
              onChange(checked ? DEFAULT_STEMS : undefined)
            }
          />
          <FieldLabel htmlFor="export-stems">
            Export mixer tracks as stems
          </FieldLabel>
        </Field>
        {value && (
          <>
            <Field data-disabled={disabled || undefined}>
              <FieldLabel htmlFor="export-stem-mode">
                Each stem contains
              </FieldLabel>
              <Choice
                id="export-stem-mode"
                value={value.mode}
                options={[
                  { value: "trackOutputs", label: "The track output" },
                  { value: "toMaster", label: "The source through the master" },
                ]}
                disabled={disabled}
                onChange={(mode) => set("mode", mode)}
              />
              <FieldDescription>
                {value.mode === "trackOutputs"
                  ? "After track effects and faders. Buses are included; their stems can overlap with the tracks feeding them."
                  : "Includes bus and master processing. Compressors and limiters react to each stem separately."}
              </FieldDescription>
            </Field>
            <Field data-disabled={disabled || undefined}>
              <FieldLabel htmlFor="export-stem-selection">
                Mixer tracks
              </FieldLabel>
              <Choice
                id="export-stem-selection"
                value={value.tracks === undefined ? "all" : "selected"}
                options={[
                  { value: "all", label: "All tracks that have sound" },
                  { value: "selected", label: "Choose tracks" },
                ]}
                disabled={disabled}
                onChange={(selection) =>
                  set(
                    "tracks",
                    selection === "all"
                      ? undefined
                      : tracks.map((track) => track.id)
                  )
                }
              />
            </Field>
            {value.tracks !== undefined && (
              <FieldSet>
                <FieldLegend variant="label">Selected tracks</FieldLegend>
                <FieldGroup className="max-h-36 gap-2 overflow-y-auto">
                  {tracks.map((track) => (
                    <Field
                      key={track.id}
                      orientation="horizontal"
                      data-disabled={disabled || undefined}
                    >
                      <Checkbox
                        id={`export-track-${track.id}`}
                        checked={value.tracks?.includes(track.id)}
                        disabled={disabled}
                        onCheckedChange={(checked) =>
                          set(
                            "tracks",
                            checked
                              ? [...(value.tracks ?? []), track.id]
                              : value.tracks?.filter((id) => id !== track.id)
                          )
                        }
                      />
                      <FieldLabel htmlFor={`export-track-${track.id}`}>
                        {track.name}
                      </FieldLabel>
                    </Field>
                  ))}
                </FieldGroup>
              </FieldSet>
            )}
            <FieldGroup className="gap-2">
              <Field
                orientation="horizontal"
                data-disabled={disabled || undefined}
              >
                <Checkbox
                  id="export-stem-mix"
                  checked={value.includeMix}
                  disabled={disabled}
                  onCheckedChange={(checked) => set("includeMix", checked)}
                />
                <FieldLabel htmlFor="export-stem-mix">
                  Include the full mix
                </FieldLabel>
              </Field>
              <Field
                orientation="horizontal"
                data-disabled={disabled || undefined}
              >
                <Checkbox
                  id="export-stem-numbered"
                  checked={value.numbered}
                  disabled={disabled}
                  onCheckedChange={(checked) => set("numbered", checked)}
                />
                <FieldLabel htmlFor="export-stem-numbered">
                  Number files in mixer order
                </FieldLabel>
              </Field>
              <Field
                orientation="horizontal"
                data-disabled={disabled || undefined}
              >
                <Checkbox
                  id="export-stem-folder"
                  checked={value.folder}
                  disabled={disabled}
                  onCheckedChange={(checked) => set("folder", checked)}
                />
                <FieldLabel htmlFor="export-stem-folder">
                  Put stems in their own folder
                </FieldLabel>
              </Field>
            </FieldGroup>
          </>
        )}
      </FieldGroup>
    </FieldSet>
  )
}
