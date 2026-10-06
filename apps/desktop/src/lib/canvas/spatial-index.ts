import { GEOMETRY_STRIDE, RECT_SELECTED, type RectBatch } from "./rect-batch"

/**
 * Time buckets over a batch sorted by start. Each bucket lists every rect
 * that overlaps it, so a point query reads one bucket and a range query
 * reads only the buckets it touches, whatever the total rect count.
 */
export interface TimeIndex {
  readonly originTick: number
  readonly bucketTicks: number
  readonly bucketCount: number
  /** Bucket `b` owns `entries[offsets[b] .. offsets[b + 1])`, ascending. */
  readonly offsets: Uint32Array
  readonly entries: Uint32Array
  /** Lowest batch index that overlaps bucket `b` or starts after it. */
  readonly firstIndex: Uint32Array
}

/** A batch sorted by start plus the index built from it. */
export interface IndexedBatch {
  readonly batch: RectBatch
  readonly index: TimeIndex
}

export interface IndexRange {
  /** First index, inclusive. */
  readonly first: number
  /** Last index, exclusive. */
  readonly last: number
}

const TARGET_PER_BUCKET = 16

/** Sorts the batch by start and builds its index. Call again after edits. */
export function indexBatch(
  batch: RectBatch,
  bucketTicks?: number
): IndexedBatch {
  batch.sortByStart()
  return { batch, index: buildTimeIndex(batch, bucketTicks) }
}

function extent(geometry: Int32Array, index: number): number {
  // A zero-length rect still has to be findable.
  return Math.max(1, geometry[index * GEOMETRY_STRIDE + 1])
}

export function buildTimeIndex(
  batch: RectBatch,
  bucketTicks?: number
): TimeIndex {
  const count = batch.count
  const geometry = batch.geometry
  if (count === 0) {
    return {
      originTick: 0,
      bucketTicks: 1,
      bucketCount: 0,
      offsets: new Uint32Array(1),
      entries: new Uint32Array(0),
      firstIndex: new Uint32Array(0),
    }
  }

  const originTick = geometry[0]
  let maxEnd = originTick + 1
  let totalLength = 0
  for (let i = 0; i < count; i++) {
    const length = extent(geometry, i)
    const end = geometry[i * GEOMETRY_STRIDE] + length
    if (end > maxEnd) maxEnd = end
    totalLength += length
  }
  const span = maxEnd - originTick

  // Buckets no narrower than the average rect keep the entry list near one
  // entry per rect even when rects are long (playlist clips).
  const size = Math.max(
    1,
    Math.ceil(
      bucketTicks ??
        Math.max((span * TARGET_PER_BUCKET) / count, totalLength / count)
    )
  )
  const bucketCount = Math.max(1, Math.ceil(span / size))

  const offsets = new Uint32Array(bucketCount + 1)
  for (let i = 0; i < count; i++) {
    const start = geometry[i * GEOMETRY_STRIDE] - originTick
    const b0 = Math.floor(start / size)
    const b1 = Math.floor((start + extent(geometry, i) - 1) / size)
    for (let b = b0; b <= b1; b++) offsets[b + 1]++
  }
  for (let b = 0; b < bucketCount; b++) offsets[b + 1] += offsets[b]

  const entries = new Uint32Array(offsets[bucketCount])
  const cursor = offsets.slice(0, bucketCount)
  for (let i = 0; i < count; i++) {
    const start = geometry[i * GEOMETRY_STRIDE] - originTick
    const b0 = Math.floor(start / size)
    const b1 = Math.floor((start + extent(geometry, i) - 1) / size)
    for (let b = b0; b <= b1; b++) entries[cursor[b]++] = i
  }

  const firstIndex = new Uint32Array(bucketCount)
  let next = count
  for (let b = bucketCount - 1; b >= 0; b--) {
    if (offsets[b + 1] > offsets[b]) next = entries[offsets[b]]
    firstIndex[b] = next
  }

  return {
    originTick,
    bucketTicks: size,
    bucketCount,
    offsets,
    entries,
    firstIndex,
  }
}

/** First index whose start is at or after `tick`. */
export function lowerBoundStart(batch: RectBatch, tick: number): number {
  const geometry = batch.geometry
  let low = 0
  let high = batch.count
  while (low < high) {
    const mid = (low + high) >>> 1
    if (geometry[mid * GEOMETRY_STRIDE] < tick) low = mid + 1
    else high = mid
  }
  return low
}

/**
 * The contiguous index range that contains every rect overlapping
 * `[tickStart, tickEnd)`. It can include rects just left of the range that
 * share its first bucket; it never misses one. Rows are not culled here.
 */
export function visibleRange(
  items: IndexedBatch,
  tickStart: number,
  tickEnd: number
): IndexRange {
  const { batch, index } = items
  if (batch.count === 0 || tickEnd <= tickStart) return { first: 0, last: 0 }
  const last = lowerBoundStart(batch, tickEnd)
  const bucket = Math.floor((tickStart - index.originTick) / index.bucketTicks)
  let first: number
  if (bucket < 0) first = 0
  else if (bucket >= index.bucketCount) first = batch.count
  else first = index.firstIndex[bucket]
  return { first: Math.min(first, last), last }
}

/**
 * The rect under a point, or -1. When rects overlap, a selected one wins,
 * then the one drawn last. Coordinates may be fractional.
 */
export function queryPoint(
  items: IndexedBatch,
  tick: number,
  row: number
): number {
  const { batch, index } = items
  const bucket = Math.floor((tick - index.originTick) / index.bucketTicks)
  if (bucket < 0 || bucket >= index.bucketCount) return -1
  const geometry = batch.geometry
  const flags = batch.flags
  let found = -1
  for (let e = index.offsets[bucket + 1] - 1; e >= index.offsets[bucket]; e--) {
    const i = index.entries[e]
    const g = i * GEOMETRY_STRIDE
    const start = geometry[g]
    if (tick < start || tick >= start + extent(geometry, i)) continue
    const top = geometry[g + 2]
    if (row < top || row >= top + geometry[g + 3]) continue
    if (flags[i] & RECT_SELECTED) return i
    if (found < 0) found = i
  }
  return found
}

/**
 * Every rect that overlaps the half-open box, as ascending indices.
 */
export function queryRect(
  items: IndexedBatch,
  tickStart: number,
  tickEnd: number,
  rowStart: number,
  rowEnd: number
): number[] {
  const { batch, index } = items
  const out: number[] = []
  if (index.bucketCount === 0 || tickEnd <= tickStart || rowEnd <= rowStart) {
    return out
  }
  const size = index.bucketTicks
  const b0 = Math.max(0, Math.floor((tickStart - index.originTick) / size))
  const b1 = Math.min(
    index.bucketCount - 1,
    Math.floor((tickEnd - index.originTick) / size)
  )
  const geometry = batch.geometry
  for (let b = b0; b <= b1; b++) {
    for (let e = index.offsets[b]; e < index.offsets[b + 1]; e++) {
      const i = index.entries[e]
      const g = i * GEOMETRY_STRIDE
      const start = geometry[g]
      // A rect spanning several buckets is reported from the first one only.
      const home = Math.floor((start - index.originTick) / size)
      if (Math.max(home, b0) !== b) continue
      if (start >= tickEnd || start + extent(geometry, i) <= tickStart) continue
      const top = geometry[g + 2]
      if (top >= rowEnd || top + geometry[g + 3] <= rowStart) continue
      out.push(i)
    }
  }
  return out
}
