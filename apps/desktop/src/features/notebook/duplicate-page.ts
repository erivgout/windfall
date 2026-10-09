import type { NotebookPage } from "@/bindings"

export function duplicatePage(
  pages: readonly NotebookPage[],
  index: number,
  maxPages: number
): NotebookPage[] | null {
  if (
    !Number.isInteger(index) ||
    index < 0 ||
    index >= pages.length ||
    pages.length >= maxPages
  ) {
    return null
  }

  return [
    ...pages.slice(0, index + 1),
    { ...pages[index] },
    ...pages.slice(index + 1),
  ]
}
