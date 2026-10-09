//! Arrangement identities and references into the one audible playlist.
use super::Builder;
use crate::report::{Outcome, ReportSection};
use std::collections::{BTreeMap, BTreeSet};
use windfall_project::{ClipId, Command, PlaylistTrackId};

/// IDs returned by successful playlist commands, aligned with source items.
pub(super) struct PlaylistReferences {
    pub tracks: BTreeMap<u16, PlaylistTrackId>,
    pub items: Vec<Vec<ClipId>>,
    pub legacy_items: Vec<Vec<ClipId>>,
}

/// Reuse only identical source placements, once per occurrence. Sharing a
/// pattern/channel alone does not establish the same clip layout.
fn matching_clips<T: PartialEq>(
    items: &[T],
    selected: &[T],
    references: &[Vec<ClipId>],
) -> (Vec<ClipId>, usize) {
    let mut used = BTreeSet::new();
    let mut clips = Vec::new();
    let mut missing = 0;
    for item in items {
        let found = selected.iter().enumerate().find(|(index, candidate)| {
            !used.contains(index) && item == *candidate && !references[*index].is_empty()
        });
        if let Some((index, _)) = found {
            used.insert(index);
            clips.extend_from_slice(&references[index]);
        } else {
            missing += 1;
        }
    }
    (clips, missing)
}

fn arrangement_name(source: Option<&str>, index: u16) -> String {
    let mut name = source.unwrap_or_default().to_owned();
    name.retain(|ch| ch != '\0');
    let mut end = name.len().min(256);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name.truncate(end);
    if name.trim().is_empty() {
        format!("Arrangement {}", u32::from(index) + 1)
    } else {
        name
    }
}

impl Builder<'_> {
    pub(super) fn arrangements(&mut self, references: PlaylistReferences) {
        // Retain the legacy empty book and allocator for one-arrangement files.
        if self.flp.arrangements.len() <= 1 {
            return;
        }
        let Some(selected) = self.flp.main_arrangement() else {
            return;
        };
        let section = ReportSection::Other;
        let mut selected_id = None;
        for arrangement in &self.flp.arrangements {
            let is_selected = std::ptr::eq(arrangement, selected);
            let name = arrangement_name(arrangement.name.as_deref(), arrangement.index);
            let name_changed = arrangement.name.as_deref() != Some(name.as_str());
            let (clips, missing) = if is_selected {
                (
                    self.project()
                        .playlist
                        .clips
                        .iter()
                        .map(|clip| clip.id)
                        .collect(),
                    0,
                )
            } else {
                let (mut clips, missing) =
                    matching_clips(&arrangement.items, &selected.items, &references.items);
                let (legacy, legacy_missing) = matching_clips(
                    &arrangement.legacy_items,
                    &selected.legacy_items,
                    &references.legacy_items,
                );
                clips.extend(legacy);
                (clips, missing + legacy_missing)
            };
            let mut used: BTreeSet<u16> = arrangement.items.iter().map(|item| item.track).collect();
            used.extend(
                arrangement
                    .tracks
                    .iter()
                    .filter_map(|track| u16::try_from(track.iid.checked_sub(1)?).ok()),
            );
            if !arrangement.legacy_items.is_empty() {
                used.insert(0);
            }
            let tracks = used
                .iter()
                .filter_map(|index| references.tracks.get(index).copied())
                .collect();
            let id = self.create(
                section,
                "An arrangement entry",
                Command::AddArrangement {
                    name,
                    clips,
                    tracks,
                },
            );
            self.report.count(
                section,
                if id.is_none() {
                    Outcome::Dropped
                } else if name_changed || !is_selected {
                    Outcome::Approximated
                } else {
                    Outcome::Exact
                },
                1,
            );
            if name_changed && id.is_some() {
                self.report.say(section, Outcome::Approximated, "An arrangement name was given a fallback or brought within Windfall's name limits.");
            }
            if is_selected {
                selected_id = id;
            } else {
                let losses = missing + arrangement.markers.len();
                self.report.count(section, Outcome::Dropped, losses as u32);
                if losses > 0 {
                    let saved = if id.is_some() {
                        " Its name and identity are saved."
                    } else {
                        ""
                    };
                    self.report.say(section, Outcome::Dropped, format!("Arrangement {} has {missing} clip placements without matching playlist references and {} timeline markers that were left out.{saved}", u32::from(arrangement.index) + 1, arrangement.markers.len()));
                }
            }
        }
        if let Some(id) = selected_id {
            self.apply(
                section,
                "The selected arrangement identity",
                Command::SwitchArrangement { id },
            );
        }
        self.report.say(section, Outcome::Approximated, "Imported arrangement entries retain names and identities with references to matching clips and tracks in the selected playlist. Windfall playback still uses one playlist; switching arrangements does not apply alternate clip layouts, track settings, markers or meter.");
    }
}

#[cfg(test)]
mod tests;
