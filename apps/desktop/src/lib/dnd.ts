/**
 * Drag and drop between panels. The browser starts a drag with this data and
 * the channel rack accepts it, so the two only share this file.
 */

/** MIME type of an audio file dragged out of the browser. */
export const SAMPLE_DRAG_TYPE = "application/x-windfall-sample"

export type SampleDrag = {
  /** Absolute path of the audio file. */
  path: string
  name: string
}

export function setSampleDrag(event: React.DragEvent, sample: SampleDrag) {
  event.dataTransfer.setData(SAMPLE_DRAG_TYPE, JSON.stringify(sample))
  event.dataTransfer.effectAllowed = "copy"
}

/** True while a sample is being dragged over the target. */
export function hasSampleDrag(event: React.DragEvent): boolean {
  return event.dataTransfer.types.includes(SAMPLE_DRAG_TYPE)
}

/** The dropped sample, or `null` when the drop carries something else. */
export function readSampleDrag(event: React.DragEvent): SampleDrag | null {
  const raw = event.dataTransfer.getData(SAMPLE_DRAG_TYPE)
  if (!raw) return null
  try {
    const value: unknown = JSON.parse(raw)
    if (
      typeof value === "object" &&
      value !== null &&
      "path" in value &&
      "name" in value &&
      typeof value.path === "string" &&
      typeof value.name === "string"
    ) {
      return { path: value.path, name: value.name }
    }
  } catch {
    return null
  }
  return null
}
