import { describe, expect, it } from "vitest"

import type { Channel, Clip, MixerTrack, Send, TrackId } from "@/bindings"

import {
  audibility,
  channelsOfTrack,
  audioClipsOfTrack,
  feedersOf,
  heardTracks,
  maxSendCount,
  outputChoices,
  reaches,
  sendChoices,
  soloSet,
  trackInputCount,
} from "./routing"

function track(
  id: TrackId,
  output: TrackId | null,
  sends: TrackId[] = [],
  extra: Partial<MixerTrack> = {}
): MixerTrack {
  return {
    id,
    name: id === 0 ? "Master" : `Track ${id}`,
    color: 0,
    volume: 1,
    pan: 0,
    muted: false,
    solo: false,
    output: id === 0 ? null : output,
    sends: sends.map((target): Send => ({ target, gain: 1 })),
    effects: [],
    ...extra,
  }
}

function channel(id: number, mixerTrack: TrackId): Channel {
  return {
    id,
    name: `Channel ${id}`,
    color: 0,
    volume: 0.8,
    pan: 0,
    muted: false,
    solo: false,
    mixerTrack,
    source: {
      type: "sampler",
      sample: null,
      rootKey: 60,
      tune: 0,
      gain: 1,
      start: 0,
      end: 1,
      reverse: false,
      envelope: null,
      cutSelf: false,
      cutGroup: 0,
    },
  }
}

const ids = (tracks: MixerTrack[]) => tracks.map((item) => item.id)
const sorted = (set: Set<TrackId>) => [...set].sort((a, b) => a - b)
const solo = (tracks: MixerTrack[], ...soloed: TrackId[]) =>
  tracks.map((item) =>
    soloed.includes(item.id) ? { ...item, solo: true } : item
  )

// 1 -> 2 -> master, 3 -> master, and 4 only sends into 2. The same graph the
// engine tests its solo rule on.
const BUS = [
  track(0, null),
  track(1, 2),
  track(2, 0),
  track(3, 0),
  track(4, null, [2]),
]

describe("soloSet", () => {
  it("leaves everything audible when nothing is soloed", () => {
    expect(sorted(soloSet(BUS))).toEqual([0, 1, 2, 3, 4])
  })

  it("keeps the soloed track, what feeds it and its way to the master", () => {
    // 3 is the only track that neither feeds 2 nor carries it onward.
    expect(sorted(soloSet(solo(BUS, 2)))).toEqual([0, 1, 2, 4])
  })

  it("keeps the buses a soloed track passes through", () => {
    expect(sorted(soloSet(solo(BUS, 1)))).toEqual([0, 1, 2])
  })

  it("follows sends downstream as well as outputs", () => {
    // 4 has no output; its sound reaches the master through its send to 2.
    expect(sorted(soloSet(solo(BUS, 4)))).toEqual([0, 2, 4])
  })

  it("adds up several soloed tracks", () => {
    expect(sorted(soloSet(solo(BUS, 1, 3)))).toEqual([0, 1, 2, 3])
  })

  it("does not keep a sibling that shares a bus with the soloed track", () => {
    const tracks = [track(0, null), track(1, 3), track(2, 3), track(3, 0)]
    expect(sorted(soloSet(solo(tracks, 1)))).toEqual([0, 1, 3])
  })

  it("keeps a send target and everything after it", () => {
    // 1 -> master with a send to the reverb 2, which goes to the bus 3.
    const tracks = [
      track(0, null),
      track(1, 0, [2]),
      track(2, 3),
      track(3, 0),
      track(4, 0),
    ]
    expect(sorted(soloSet(solo(tracks, 1)))).toEqual([0, 1, 2, 3])
  })

  it("keeps everything that plays into a soloed master", () => {
    const tracks = [...BUS, track(5, null)]
    // 5 has no output and no sends, so it never reaches the master.
    expect(sorted(soloSet(solo(tracks, 0)))).toEqual([0, 1, 2, 3, 4])
  })

  it("ignores routing stored on the master and edges to missing tracks", () => {
    const tracks = [
      { ...track(0, null), output: 1, sends: [{ target: 2, gain: 1 }] },
      track(1, 99),
      track(2, 0),
    ]
    expect(sorted(soloSet(solo(tracks, 1)))).toEqual([1])
    expect(sorted(soloSet(solo(tracks, 0)))).toEqual([0, 2])
  })
})

describe("audibility", () => {
  it("is heard, muted or silenced", () => {
    const tracks = solo(BUS, 2)
    expect(audibility(tracks, 2)).toBe("heard")
    expect(audibility(tracks, 1)).toBe("heard")
    expect(audibility(tracks, 0)).toBe("heard")
    expect(audibility(tracks, 3)).toBe("silenced")
  })

  it("lets mute win over solo", () => {
    const tracks = solo(BUS, 2).map((item) =>
      item.id === 2 || item.id === 1 ? { ...item, muted: true } : item
    )
    expect(audibility(tracks, 2)).toBe("muted")
    expect(audibility(tracks, 1)).toBe("muted")
    // Still silenced by the solo, even though the soloed track is muted.
    expect(audibility(tracks, 3)).toBe("silenced")
  })

  it("reports a muted track when nothing is soloed", () => {
    const tracks = BUS.map((item) =>
      item.id === 3 ? { ...item, muted: true } : item
    )
    expect(audibility(tracks, 3)).toBe("muted")
    expect(audibility(tracks, 1)).toBe("heard")
  })

  it("works the solo set out once per track list", () => {
    const tracks = solo(BUS, 2)
    expect(heardTracks(tracks)).toBe(heardTracks(tracks))
    expect(heardTracks([...tracks])).not.toBe(heardTracks(tracks))
  })
})

describe("reaches", () => {
  it("follows outputs and sends", () => {
    expect(reaches(BUS, 1, 0)).toBe(true)
    expect(reaches(BUS, 4, 0)).toBe(true)
    expect(reaches(BUS, 2, 1)).toBe(false)
    expect(reaches(BUS, 0, 1)).toBe(false)
    expect(reaches(BUS, 3, 3)).toBe(true)
  })
})

describe("outputChoices", () => {
  it("offers every other track when nothing plays into the track", () => {
    expect(ids(outputChoices(BUS, 3).tracks)).toEqual([0, 1, 2, 4])
    expect(outputChoices(BUS, 3).looping).toBe(0)
  })

  it("leaves out tracks whose output leads back to the track", () => {
    // 1 plays into 2, and 4 sends into 2, so 2 cannot route to either.
    const choices = outputChoices(BUS, 2)
    expect(ids(choices.tracks)).toEqual([0, 3])
    expect(choices.looping).toBe(2)
  })

  it("leaves out tracks that reach it through a chain", () => {
    const chain = [track(0, null), track(1, 2), track(2, 3), track(3, 0)]
    expect(ids(outputChoices(chain, 3).tracks)).toEqual([0])
    expect(ids(outputChoices(chain, 2).tracks)).toEqual([0, 3])
    expect(ids(outputChoices(chain, 1).tracks)).toEqual([0, 2, 3])
  })

  it("always offers the master and never offers the track itself", () => {
    for (const item of BUS.slice(1)) {
      const offered = ids(outputChoices(BUS, item.id).tracks)
      expect(offered).toContain(0)
      expect(offered).not.toContain(item.id)
    }
  })

  it("offers nothing for the master", () => {
    expect(outputChoices(BUS, 0)).toEqual({ tracks: [], looping: 0 })
  })
})

describe("sendChoices", () => {
  it("leaves out targets that would loop and sends that exist", () => {
    // 4 already sends to 2.
    expect(ids(sendChoices(BUS, 4).tracks)).toEqual([0, 1, 3])
    // 2 is fed by 1 and 4.
    const choices = sendChoices(BUS, 2)
    expect(ids(choices.tracks)).toEqual([0, 3])
    expect(choices.looping).toBe(2)
  })

  it("offers nothing for a track that does not exist", () => {
    expect(sendChoices(BUS, 99).tracks).toEqual([])
  })
})

describe("what plays into a track", () => {
  const channels = [channel(10, 1), channel(11, 2), channel(12, 1)]

  const audioClip = (id: number, mixerTrack: number, start = 0) =>
    ({
      id,
      start,
      content: { type: "audio", mixerTrack },
    }) as unknown as Clip
  const clips = [
    audioClip(21, 2, 960),
    audioClip(20, 2, 0),
    audioClip(22, 3),
    {
      id: 23,
      start: 0,
      content: { type: "pattern", pattern: 1 },
    } as unknown as Clip,
  ]

  it("lists channels, audio clips, outputs and sends", () => {
    const feeders = feedersOf(BUS, channels, clips, 2)
    expect(feeders.channels.map((item) => item.id)).toEqual([11])
    expect(feeders.clips).toEqual([20, 21])
    expect(ids(feeders.outputs)).toEqual([1])
    expect(ids(feeders.sends)).toEqual([4])
  })

  it("groups audio clips by the track they play into, in timeline order", () => {
    expect(audioClipsOfTrack(clips, 2)).toEqual([20, 21])
    expect(audioClipsOfTrack(clips, 3)).toEqual([22])
    // The same list while no clip changes, so a strip does not render again.
    expect(audioClipsOfTrack(clips, 2)).toBe(audioClipsOfTrack(clips, 2))
    expect(audioClipsOfTrack(clips, 1)).toEqual([])
    expect(audioClipsOfTrack(clips, 1)).toBe(audioClipsOfTrack(clips, 4))
  })

  it("counts the tracks that play into a track", () => {
    expect(trackInputCount(BUS, 2)).toBe(2)
    expect(trackInputCount(BUS, 0)).toBe(2)
    expect(trackInputCount(BUS, 3)).toBe(0)
  })

  it("groups channels by track and keeps the lists while nothing changes", () => {
    expect(channelsOfTrack(channels, 1).map((item) => item.id)).toEqual([
      10, 12,
    ])
    expect(channelsOfTrack(channels, 1)).toBe(channelsOfTrack(channels, 1))
    expect(channelsOfTrack(channels, 3)).toEqual([])
    expect(channelsOfTrack(channels, 3)).toBe(channelsOfTrack(channels, 4))
  })

  it("finds the most sends any track has", () => {
    expect(maxSendCount(BUS)).toBe(1)
    expect(maxSendCount([track(0, null), track(1, 0, [0, 2, 3])])).toBe(3)
    expect(maxSendCount([])).toBe(0)
  })
})
