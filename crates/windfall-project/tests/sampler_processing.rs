use windfall_project::*;

fn document() -> (Document, ChannelId) {
    let mut document = Document::new(Project::new("sampler"));
    let result = document
        .dispatch(
            Command::AddChannel {
                name: None,
                sample: None,
                instrument: None,
                index: None,
                mixer_track: None,
            },
            None,
        )
        .unwrap();
    (document, ChannelId(result.created[0]))
}
fn setting() -> SamplerStretch {
    SamplerStretch::Spectral {
        ratio: 2.0,
        quality: ClipStretchQuality::High,
        formants: true,
        range: SamplerKeyRange::around_root(60),
    }
}
fn command(id: ChannelId, stretch: SamplerStretch) -> Command {
    Command::UpdateSampler {
        id,
        patch: SamplerPatch {
            stretch: Some(stretch),
            ..Default::default()
        },
    }
}

#[test]
fn sampler_processing_legacy_omits_tape_and_defaults_without_a_format_change() {
    let (document, _) = document();
    let json = file::to_json(document.project()).unwrap();
    assert!(!json.contains("stretch"));
    let loaded = file::from_json(&json).unwrap();
    assert_eq!(&loaded, document.project());
    assert_eq!(loaded.format_version, 1);
    let ChannelSource::Sampler(settings) = &loaded.channels[0].source else {
        panic!()
    };
    assert_eq!(settings.stretch, SamplerStretch::Tape);
    for root in 0..=127 {
        let range = SamplerKeyRange::around_root(root);
        assert_eq!(range.last - range.first, 11);
        assert!(range.first <= root && range.last >= root);
    }
}

#[test]
fn sampler_processing_apply_is_one_checked_undo_clone_and_save_load_step() {
    let (mut document, id) = document();
    let before = document.project().clone();
    let cursor = document.history().cursor;
    document.dispatch(command(id, setting()), None).unwrap();
    let after = document.project().clone();
    assert_eq!(document.history().cursor, cursor + 1);
    document.dispatch(command(id, setting()), None).unwrap();
    assert_eq!(document.history().cursor, cursor + 1);
    document.undo().unwrap();
    assert_eq!(document.project(), &before);
    document.redo().unwrap();
    assert_eq!(document.project(), &after);
    document
        .dispatch(Command::DuplicateChannel { id }, None)
        .unwrap();
    assert_eq!(
        document.project().channels[0].source,
        document.project().channels[1].source
    );
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("sampler.windfall");
    file::save(document.project(), &path).unwrap();
    assert_eq!(file::load(path).unwrap(), *document.project());
    let json = file::to_json(document.project()).unwrap();
    assert!(json.contains("spectral"));
    assert!(!json.contains("variants") && !json.contains("rendered"));
}

#[test]
fn sampler_processing_invalid_duration_and_ranges_preserve_document_and_history() {
    let (mut document, id) = document();
    for (ratio, first, last) in [
        (f64::NAN, 60, 60),
        (f64::INFINITY, 60, 60),
        (0.249, 60, 60),
        (4.01, 60, 60),
        (1.0, 62, 61),
        (1.0, 0, 128),
    ] {
        let bad = SamplerStretch::Spectral {
            ratio,
            quality: ClipStretchQuality::Fast,
            formants: false,
            range: SamplerKeyRange { first, last },
        };
        let before = document.snapshot(None);
        assert!(document.dispatch(command(id, bad), None).is_err());
        assert_eq!(document.snapshot(None), before);
        let mut project = document.project().clone();
        let ChannelSource::Sampler(settings) = &mut project.channels[0].source else {
            panic!()
        };
        settings.stretch = bad;
        assert!(project.check().is_err());
        if ratio.is_finite() {
            assert!(file::from_json(&file::to_json(&project).unwrap()).is_err());
        }
    }
}
