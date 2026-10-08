use windfall_project::timeline::MeterMap;
use windfall_project::{
    MAX_SONG_TICKS, MeterChange, MeterChangeId, MusicalPosition, TimeSignature,
};

fn signature(numerator: u8, denominator: u8) -> TimeSignature {
    TimeSignature {
        numerator,
        denominator,
    }
}

#[test]
fn shared_ui_fixtures_prove_rust_positions_and_inverse() {
    #[derive(serde::Deserialize)]
    struct Fixture {
        legacy: TimeSignature,
        meters: Vec<MeterChange>,
        positions: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        tick: u32,
        position: MusicalPosition,
    }
    let fixture: Fixture = serde_json::from_str(include_str!("meter-fixtures.json")).unwrap();
    let map = MeterMap::new(fixture.legacy, &fixture.meters).unwrap();
    for entry in fixture.positions {
        assert_eq!(map.tick_to_position(entry.tick).unwrap(), entry.position);
        assert_eq!(map.position_to_tick(entry.position).unwrap(), entry.tick);
    }
}

#[test]
fn unaligned_nonzero_changes_have_shortened_bars_and_checked_inverse() {
    let map = MeterMap::new(
        signature(4, 4),
        &[
            MeterChange {
                id: MeterChangeId(2),
                tick: 4_001,
                signature: signature(7, 8),
            },
            MeterChange {
                id: MeterChangeId(3),
                tick: 7_400,
                signature: signature(3, 4),
            },
        ],
    )
    .unwrap();
    for (tick, bar, beat, within) in [
        (0, 1, 1, 0),
        (3_840, 2, 1, 0),
        (4_000, 2, 1, 160),
        (4_001, 3, 1, 0),
        (4_481, 3, 2, 0),
        (7_360, 3, 7, 479),
        (7_361, 4, 1, 0),
        (7_399, 4, 1, 38),
        (7_400, 5, 1, 0),
        (8_360, 5, 2, 0),
        (10_280, 6, 1, 0),
    ] {
        let position = MusicalPosition {
            bar,
            beat,
            tick: within,
        };
        assert_eq!(map.tick_to_position(tick).unwrap(), position);
        assert_eq!(map.position_to_tick(position).unwrap(), tick);
    }
    assert!(
        map.position_to_tick(MusicalPosition {
            bar: 2,
            beat: 1,
            tick: 161
        })
        .is_err()
    );
    assert!(
        map.position_to_tick(MusicalPosition {
            bar: 4,
            beat: 1,
            tick: 39
        })
        .is_err()
    );
    assert!(
        map.position_to_tick(MusicalPosition {
            bar: 5,
            beat: 4,
            tick: 0
        })
        .is_err()
    );
    let last = map.tick_to_position(MAX_SONG_TICKS).unwrap();
    assert_eq!(map.position_to_tick(last).unwrap(), MAX_SONG_TICKS);
    assert!(
        map.position_to_tick(MusicalPosition {
            bar: u32::MAX,
            beat: 1,
            tick: 0
        })
        .is_err()
    );
}

#[test]
fn invalid_and_colliding_meter_ticks_never_divide_by_zero() {
    assert!(MeterMap::new(signature(4, 0), &[]).is_err());
    assert!(
        MeterMap::new(
            signature(4, 4),
            &[
                MeterChange {
                    id: MeterChangeId(2),
                    tick: 100,
                    signature: signature(7, 8)
                },
                MeterChange {
                    id: MeterChangeId(3),
                    tick: 100,
                    signature: signature(3, 4)
                },
            ]
        )
        .is_err()
    );
    let map = MeterMap::new(
        signature(3, 8),
        &[MeterChange {
            id: MeterChangeId(2),
            tick: 0,
            signature: signature(7, 16),
        }],
    )
    .unwrap();
    assert_eq!(map.tick_to_position(1_680).unwrap().bar, 2);
}
