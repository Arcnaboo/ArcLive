//! Integration: engine process without opening a device window.

use arclive_core::{QuantizeGrid, Song};
use arclive_engine::{Engine, EngineCommand, EngineConfig, PublishedGraph};

#[test]
fn offline_play_stop_seek() {
    let g = PublishedGraph::from_song(&Song::new_demo());
    let (mut engine, mut handle) = Engine::create(EngineConfig::default(), g);
    engine.test_tone = true;
    let _ = handle.command_tx.push(EngineCommand::SeekSamples(0));
    let _ = handle.command_tx.push(EngineCommand::Play);
    let mut buf = vec![0.0_f32; 256 * 2];
    engine.process(&mut buf, 256, 2);
    assert!(engine.sample_position >= 256);
    let _ = handle.command_tx.push(EngineCommand::Stop);
    engine.process(&mut buf, 256, 2);
}

#[test]
fn session_launch_quantized() {
    let g = PublishedGraph::from_song(&Song::new_demo());
    let (mut engine, mut handle) = Engine::create(EngineConfig::default(), g);
    engine.test_tone = false;
    let _ = handle.command_tx.push(EngineCommand::Play);
    let _ = handle.command_tx.push(EngineCommand::LaunchClip {
        track_index: 0,
        slot_index: 0,
        quantize: QuantizeGrid::None,
    });
    let mut buf = vec![0.0_f32; 2048 * 2];
    engine.process(&mut buf, 2048, 2);
    let peak = buf.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    assert!(peak > 0.01, "peak {peak}");
}
