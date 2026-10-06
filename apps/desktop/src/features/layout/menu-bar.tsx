import { ActionMenuItem } from "@/components/action-menu-item"
import {
  Menubar,
  MenubarContent,
  MenubarItem,
  MenubarMenu,
  MenubarSeparator,
  MenubarSub,
  MenubarSubContent,
  MenubarSubTrigger,
  MenubarTrigger,
} from "@/components/ui/menubar"
import { useActions, useAppState } from "@/lib/actions"
import { MENUS, type MenuEntry } from "@/lib/actions/menus"

// Rendered only while its menu is open, so it can follow the whole app state
// without costing anything the rest of the time.
function MenuEntries({ entries }: { entries: MenuEntry[] }) {
  const actions = useActions()
  const state = useAppState()
  const find = (id: string) => actions.find((action) => action.id === id)
  const hasToggles = entries.some(
    (entry) => typeof entry === "string" && find(entry)?.checked !== undefined
  )

  return entries.map((entry, index) => {
    if (typeof entry === "string") {
      const action = find(entry)
      return (
        action && (
          <ActionMenuItem
            key={entry}
            action={action}
            state={state}
            inset={hasToggles}
          />
        )
      )
    }
    if ("separator" in entry) return <MenubarSeparator key={index} />

    const items = actions.filter((action) => action.section === entry.section)
    return (
      <MenubarSub key={entry.submenu}>
        <MenubarSubTrigger>{entry.submenu}</MenubarSubTrigger>
        <MenubarSubContent className="w-auto min-w-48">
          {items.length === 0 ? (
            <MenubarItem disabled>{entry.empty}</MenubarItem>
          ) : (
            items.map((action) => (
              <ActionMenuItem key={action.id} action={action} state={state} />
            ))
          )}
        </MenubarSubContent>
      </MenubarSub>
    )
  })
}

/** File, Edit, View, Options and Help. Every item is a registry action. */
export function AppMenuBar() {
  return (
    <Menubar className="h-full gap-0 rounded-none border-0 p-0">
      {MENUS.map((menu) => (
        <MenubarMenu key={menu.title}>
          <MenubarTrigger className="h-6 font-normal text-foreground/85 aria-expanded:text-foreground">
            {menu.title}
          </MenubarTrigger>
          <MenubarContent className="w-auto min-w-56" sideOffset={4}>
            <MenuEntries entries={menu.entries} />
          </MenubarContent>
        </MenubarMenu>
      ))}
    </Menubar>
  )
}
