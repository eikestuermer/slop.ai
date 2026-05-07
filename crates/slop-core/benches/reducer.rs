//! Performance benchmarks for the slop-core hot paths.
//!
//! Run: `cargo bench -p slop-core`.
//! Targets: the reducer's apply path (we apply tens of thousands of ops on
//! load) and the validator (runs on every plan).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use slop_core::{
    ids, reducer, Asset, ClipItem, ClipMetadata, Op, OpKind, Resolution, Timeline, TrackKind,
};

fn bench_apply_n_clips(c: &mut Criterion) {
    let mut group = c.benchmark_group("reducer/apply");
    for n in [100usize, 1_000, 10_000] {
        group.bench_function(format!("clips_{n}"), |b| {
            b.iter(|| {
                let mut tl = Timeline::empty();
                let aid = "a1".to_string();
                let tid = ids::track();
                reducer::apply(
                    &mut tl,
                    &Op::new(OpKind::AddAsset(Asset {
                        asset_id: aid.clone(),
                        uri: "file:///x.mp4".into(),
                        duration_sec: 1_000_000.0,
                        has_video: true,
                        has_audio: true,
                        fps: Some(30.0),
                        resolution: Some(Resolution { w: 1920, h: 1080 }),
                        transcript_ref: None,
                        shot_list_ref: None,
                    })),
                )
                .unwrap();
                reducer::apply(
                    &mut tl,
                    &Op::new(OpKind::AddTrack {
                        track_id: tid.clone(),
                        kind: TrackKind::Video,
                    }),
                )
                .unwrap();
                for i in 0..n {
                    let item_id = format!("c_{i}");
                    let t_in = i as f64 * 1.0;
                    let item = ClipItem {
                        item_id,
                        asset_id: aid.clone(),
                        src_in: i as f64,
                        src_out: i as f64 + 0.9,
                        timeline_in: t_in,
                        timeline_out: t_in + 0.9,
                        speed: 1.0,
                        effects: vec![],
                        markers: vec![],
                        metadata: ClipMetadata::default(),
                    };
                    reducer::apply(
                        &mut tl,
                        &Op::new(OpKind::InsertClip {
                            track_id: tid.clone(),
                            clip: item,
                        }),
                    )
                    .unwrap();
                }
                black_box(tl);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_apply_n_clips);
criterion_main!(benches);
