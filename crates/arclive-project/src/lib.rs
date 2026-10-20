//! Versioned `.arclive` project documents.

use arclive_core::{
    ArrangementClip, AudioClipData, ClipSlot, DeviceSpec, MidiClipData, MidiNote, MixerChannel,
    QuantizeGrid, Scene, Song, TimeSignature, Track, TrackKind, Transport, WarpMarker, WarpMode,
    WorkspaceView,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const CURRENT_SCHEMA: u32 = 2;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported schema version {0}")]
    UnsupportedVersion(u32),
    #[error("invalid project: {0}")]
    Invalid(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub schema_version: u32,
    pub song: SongDto,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SongDto {
    pub name: String,
    pub time_signature: TimeSignature,
    pub transport: Transport,
    pub tracks: Vec<TrackDto>,
    pub scenes: Vec<Scene>,
    pub arrangement: Vec<ArrangementClipDto>,
    pub launch_quantize: QuantizeGrid,
    pub workspace: WorkspaceView,
    pub arrangement_mode: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackDto {
    pub id: u64,
    pub name: String,
    pub kind: TrackKind,
    pub mixer: MixerChannel,
    pub devices: Vec<DeviceSpec>,
    pub slots: Vec<SlotDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SlotDto {
    Empty,
    Audio {
        id: u64,
        name: String,
        length_beats: f64,
        channels: u16,
        sample_rate: f32,
        asset_path: Option<String>,
        missing_asset: bool,
        #[serde(default)]
        warp_enabled: bool,
        #[serde(default)]
        warp_mode: WarpMode,
        #[serde(default)]
        warp_markers: Vec<WarpMarker>,
        #[serde(default)]
        transpose_semitones: i8,
        #[serde(default)]
        detune_cents: i16,
        #[serde(default)]
        loop_on: bool,
        #[serde(default)]
        loop_start_beats: f64,
        #[serde(default)]
        loop_end_beats: f64,
        #[serde(default)]
        clip_start_beats: f64,
        #[serde(default)]
        clip_end_beats: f64,
    },
    Midi {
        id: u64,
        name: String,
        length_beats: f64,
        notes: Vec<MidiNote>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArrangementClipDto {
    pub id: u64,
    pub track_id: u64,
    pub name: String,
    pub start_beats: f64,
    pub length_beats: f64,
    pub channels: u16,
    pub sample_rate: f32,
    pub asset_path: Option<String>,
    pub missing_asset: bool,
}

impl ProjectDocument {
    pub fn from_song(song: &Song) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA,
            song: song_to_dto(song),
        }
    }

    pub fn into_song(self, project_root: &Path) -> Result<Song, ProjectError> {
        let doc = migrate(self)?;
        Ok(dto_to_song(doc.song, project_root))
    }
}

fn migrate(mut doc: ProjectDocument) -> Result<ProjectDocument, ProjectError> {
    while doc.schema_version < CURRENT_SCHEMA {
        match doc.schema_version {
            // v1 -> v2 adds warp fields (serde defaults).
            0 => doc.schema_version = 1,
            1 => doc.schema_version = 2,
            v => return Err(ProjectError::UnsupportedVersion(v)),
        }
    }
    if doc.schema_version > CURRENT_SCHEMA {
        return Err(ProjectError::UnsupportedVersion(doc.schema_version));
    }
    Ok(doc)
}

/// Migrate a synthetic older document for tests (v0 → v1).
pub fn migrate_v0_to_v1(mut value: serde_json::Value) -> Result<ProjectDocument, ProjectError> {
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if version == 0
        && let Some(obj) = value.as_object_mut()
    {
        obj.insert("schema_version".into(), serde_json::json!(1));
        // v0 used `title` instead of song.name
        if let Some(song) = obj.get_mut("song").and_then(|s| s.as_object_mut())
            && let Some(title) = song.remove("title")
        {
            song.insert("name".into(), title);
        }
    }
    let doc: ProjectDocument = serde_json::from_value(value)?;
    migrate(doc)
}

pub fn save_project(dir: &Path, song: &Song) -> Result<PathBuf, ProjectError> {
    fs::create_dir_all(dir)?;
    fs::create_dir_all(dir.join("assets"))?;
    let doc = ProjectDocument::from_song(song);
    let path = dir.join("project.json");
    let text = serde_json::to_string_pretty(&doc)?;
    fs::write(&path, text)?;
    Ok(path)
}

pub fn save_autosave(dir: &Path, song: &Song) -> Result<PathBuf, ProjectError> {
    fs::create_dir_all(dir)?;
    let doc = ProjectDocument::from_song(song);
    let path = dir.join("project.autosave.json");
    fs::write(&path, serde_json::to_string_pretty(&doc)?)?;
    Ok(path)
}

pub fn open_project(dir: &Path) -> Result<Song, ProjectError> {
    let path = dir.join("project.json");
    let text = fs::read_to_string(&path)?;
    let doc: ProjectDocument = serde_json::from_str(&text)?;
    doc.into_song(dir)
}

fn song_to_dto(song: &Song) -> SongDto {
    SongDto {
        name: song.name.clone(),
        time_signature: song.time_signature,
        transport: song.transport.clone(),
        tracks: song.tracks.iter().map(track_to_dto).collect(),
        scenes: song.scenes.clone(),
        arrangement: song.arrangement.iter().map(arr_to_dto).collect(),
        launch_quantize: song.launch_quantize,
        workspace: song.workspace,
        arrangement_mode: song.arrangement_mode,
    }
}

fn track_to_dto(t: &Track) -> TrackDto {
    TrackDto {
        id: t.id.0,
        name: t.name.clone(),
        kind: t.kind,
        mixer: t.mixer.clone(),
        devices: t.devices.clone(),
        slots: t.slots.iter().map(slot_to_dto).collect(),
    }
}

fn slot_to_dto(s: &ClipSlot) -> SlotDto {
    match s {
        ClipSlot::Empty => SlotDto::Empty,
        ClipSlot::Audio(c) => SlotDto::Audio {
            id: c.id.0,
            name: c.name.clone(),
            length_beats: c.length_beats,
            channels: c.channels,
            sample_rate: c.sample_rate,
            asset_path: c.asset_path.clone(),
            missing_asset: c.missing_asset,
            warp_enabled: c.warp_enabled,
            warp_mode: c.warp_mode,
            warp_markers: c.warp_markers.clone(),
            transpose_semitones: c.transpose_semitones,
            detune_cents: c.detune_cents,
            loop_on: c.loop_on,
            loop_start_beats: c.loop_start_beats,
            loop_end_beats: c.loop_end_beats,
            clip_start_beats: c.clip_start_beats,
            clip_end_beats: c.clip_end_beats,
        },
        ClipSlot::Midi(c) => SlotDto::Midi {
            id: c.id.0,
            name: c.name.clone(),
            length_beats: c.length_beats,
            notes: c.notes.clone(),
        },
    }
}

fn arr_to_dto(c: &ArrangementClip) -> ArrangementClipDto {
    ArrangementClipDto {
        id: c.id.0,
        track_id: c.track_id.0,
        name: c.name.clone(),
        start_beats: c.start_beats,
        length_beats: c.length_beats,
        channels: c.channels,
        sample_rate: c.sample_rate,
        asset_path: c.asset_path.clone(),
        missing_asset: c.missing_asset,
    }
}

fn dto_to_song(dto: SongDto, root: &Path) -> Song {
    let tracks = dto
        .tracks
        .into_iter()
        .map(|t| dto_to_track(t, root))
        .collect();
    let arrangement = dto
        .arrangement
        .into_iter()
        .map(|c| dto_to_arr(c, root))
        .collect();
    Song {
        name: dto.name,
        time_signature: dto.time_signature,
        transport: dto.transport,
        tracks,
        scenes: dto.scenes,
        arrangement,
        launch_quantize: dto.launch_quantize,
        workspace: dto.workspace,
        arrangement_mode: dto.arrangement_mode,
    }
}

fn dto_to_track(t: TrackDto, root: &Path) -> Track {
    Track {
        id: arclive_core::TrackId::from_raw(t.id),
        name: t.name,
        kind: t.kind,
        mixer: t.mixer,
        devices: t.devices,
        slots: t.slots.into_iter().map(|s| dto_to_slot(s, root)).collect(),
        playing_slot: None,
    }
}

fn dto_to_slot(s: SlotDto, root: &Path) -> ClipSlot {
    match s {
        SlotDto::Empty => ClipSlot::Empty,
        SlotDto::Audio {
            id,
            name,
            length_beats,
            channels,
            sample_rate,
            asset_path,
            mut missing_asset,
            warp_enabled,
            warp_mode,
            warp_markers,
            transpose_semitones,
            detune_cents,
            loop_on,
            loop_start_beats,
            loop_end_beats,
            clip_start_beats,
            clip_end_beats,
        } => {
            let mut samples = Vec::new();
            if let Some(rel) = &asset_path {
                let path = root.join(rel);
                match read_raw_pcm_f32(&path) {
                    Ok(pcm) => samples = pcm,
                    Err(_) => missing_asset = true,
                }
            }
            let mut clip = AudioClipData {
                id: arclive_core::ClipId::from_raw(id),
                name,
                length_beats,
                samples,
                channels,
                sample_rate,
                asset_path,
                missing_asset,
                warp_enabled,
                warp_mode,
                warp_markers,
                transpose_semitones,
                detune_cents,
                loop_on,
                loop_start_beats,
                loop_end_beats,
                clip_start_beats,
                clip_end_beats,
            };
            if clip.warp_markers.is_empty() {
                clip.init_default_markers();
            }
            ClipSlot::Audio(clip)
        }
        SlotDto::Midi {
            id,
            name,
            length_beats,
            notes,
        } => ClipSlot::Midi(MidiClipData {
            id: arclive_core::ClipId::from_raw(id),
            name,
            length_beats,
            notes,
        }),
    }
}

fn dto_to_arr(c: ArrangementClipDto, root: &Path) -> ArrangementClip {
    let mut samples = Vec::new();
    let mut missing = c.missing_asset;
    if let Some(rel) = &c.asset_path {
        match read_raw_pcm_f32(&root.join(rel)) {
            Ok(pcm) => samples = pcm,
            Err(_) => missing = true,
        }
    }
    ArrangementClip {
        id: arclive_core::ClipId::from_raw(c.id),
        track_id: arclive_core::TrackId::from_raw(c.track_id),
        name: c.name,
        start_beats: c.start_beats,
        length_beats: c.length_beats,
        samples,
        channels: c.channels,
        sample_rate: c.sample_rate,
        asset_path: c.asset_path,
        missing_asset: missing,
    }
}

/// Minimal raw little-endian f32 PCM asset reader (no WAV header).
fn read_raw_pcm_f32(path: &Path) -> Result<Vec<f32>, ProjectError> {
    let bytes = fs::read(path)?;
    if !bytes.len().is_multiple_of(4) {
        return Err(ProjectError::Invalid("pcm size".into()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn roundtrip_demo() {
        let dir = env::temp_dir().join(format!("arclive_proj_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let song = Song::new_demo();
        save_project(&dir, &song).unwrap();
        let loaded = open_project(&dir).unwrap();
        assert_eq!(loaded.name, song.name);
        assert_eq!(loaded.tracks.len(), song.tracks.len());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrate_v0() {
        let v0 = serde_json::json!({
            "schema_version": 0,
            "song": {
                "title": "Old",
                "time_signature": { "numerator": 4, "denominator": 4 },
                "transport": {
                    "state": "Stopped",
                    "sample_position": 0,
                    "bpm": 120.0,
                    "loop_enabled": false,
                    "loop_start_beats": 0.0,
                    "loop_end_beats": 16.0,
                    "metronome": false,
                    "record_armed": false
                },
                "tracks": [],
                "scenes": [],
                "arrangement": [],
                "launch_quantize": "Bar",
                "workspace": "Session",
                "arrangement_mode": false
            }
        });
        let doc = migrate_v0_to_v1(v0).unwrap();
        assert_eq!(doc.schema_version, 2);
        assert_eq!(doc.song.name, "Old");
    }

    #[test]
    fn missing_asset_flag() {
        let dir = env::temp_dir().join(format!("arclive_miss_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("assets")).unwrap();
        let mut song = Song::new_empty("m");
        if let Some(t) = song.tracks.get_mut(0) {
            t.slots[0] = ClipSlot::Audio(AudioClipData {
                id: arclive_core::ClipId::new(),
                name: "gone".into(),
                length_beats: 1.0,
                samples: vec![],
                channels: 2,
                sample_rate: 48000.0,
                asset_path: Some("assets/missing.pcm".into()),
                missing_asset: false,
                warp_enabled: false,
                warp_mode: WarpMode::Beats,
                warp_markers: vec![],
                transpose_semitones: 0,
                detune_cents: 0,
                loop_on: false,
                loop_start_beats: 0.0,
                loop_end_beats: 1.0,
                clip_start_beats: 0.0,
                clip_end_beats: 1.0,
            });
        }
        save_project(&dir, &song).unwrap();
        let loaded = open_project(&dir).unwrap();
        match &loaded.tracks[0].slots[0] {
            ClipSlot::Audio(c) => assert!(c.missing_asset),
            _ => panic!("expected audio"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_v2_warp_roundtrip() {
        let dir = env::temp_dir().join(format!("arclive_warp_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut song = Song::new_demo();
        if let Some(ClipSlot::Audio(c)) = song.tracks[0].slots.get_mut(0) {
            c.warp_enabled = true;
            c.warp_mode = WarpMode::RePitch;
            c.transpose_semitones = 2;
            c.detune_cents = -10;
            c.init_default_markers();
        }
        save_project(&dir, &song).unwrap();
        let loaded = open_project(&dir).unwrap();
        match &loaded.tracks[0].slots[0] {
            ClipSlot::Audio(c) => {
                assert!(c.warp_enabled);
                assert_eq!(c.warp_mode, WarpMode::RePitch);
                assert_eq!(c.transpose_semitones, 2);
                assert_eq!(c.detune_cents, -10);
                assert!(c.warp_markers.len() >= 2);
            }
            _ => panic!("expected audio"),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
