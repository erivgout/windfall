import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"

import { isScaleId, ROOT_NAMES, scaleDefinition, SCALES } from "./scales"
import { usePianoRollStore } from "./store"

/** Independent scale shading and pitch snapping; changing either never edits notes. */
export function ScaleControls() {
  const root = usePianoRollStore((state) => state.scaleRoot)
  const id = usePianoRollStore((state) => state.scaleId)
  const highlight = usePianoRollStore((state) => state.highlightScale)
  const snap = usePianoRollStore((state) => state.snapToScale)
  const label = `${ROOT_NAMES[root]} ${scaleDefinition(id).label}`

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant={highlight || snap ? "secondary" : "outline"}
            size="sm"
            aria-label={`Scale settings: ${label}`}
          />
        }
      >
        Scale: {ROOT_NAMES[root].split(" / ")[0]} {scaleDefinition(id).label}
        {snap ? " · snap" : ""}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-80">
        <DropdownMenuGroup>
          <DropdownMenuLabel>{label}</DropdownMenuLabel>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>Root key</DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-40">
              <DropdownMenuRadioGroup
                value={String(root)}
                onValueChange={(value) =>
                  usePianoRollStore.getState().setScaleRoot(Number(value))
                }
              >
                <DropdownMenuLabel>Root key</DropdownMenuLabel>
                {ROOT_NAMES.map((name, index) => (
                  <DropdownMenuRadioItem key={name} value={String(index)}>
                    {name}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>Musical scale</DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-64">
              <DropdownMenuRadioGroup
                value={id}
                onValueChange={(value) => {
                  if (isScaleId(value))
                    usePianoRollStore.getState().setScaleId(value)
                }}
              >
                <DropdownMenuLabel>Musical scale</DropdownMenuLabel>
                {SCALES.map((scale) => (
                  <DropdownMenuRadioItem key={scale.id} value={scale.id}>
                    {scale.label}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuCheckboxItem
            checked={highlight}
            onCheckedChange={(enabled) =>
              usePianoRollStore.getState().setHighlightScale(enabled)
            }
          >
            Highlight scale
          </DropdownMenuCheckboxItem>
          <DropdownMenuCheckboxItem
            checked={snap}
            onCheckedChange={(enabled) =>
              usePianoRollStore.getState().setSnapToScale(enabled)
            }
          >
            Snap pitches to scale
          </DropdownMenuCheckboxItem>
          <DropdownMenuLabel>
            Alt bypasses snap when drawing or dragging.
          </DropdownMenuLabel>
          <DropdownMenuLabel>
            Groups snap their anchor and keep their intervals.
          </DropdownMenuLabel>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
