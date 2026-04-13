use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct RecordingSession {
    pub mic_path: PathBuf,
    pub sys_path: PathBuf,
    _mic_stream: Stream,
    _sys_stream: Stream,
}

// Streams are not Send by default; we keep them on the thread that created them
// by storing the session inside a Mutex<Option<...>> in commands.rs
unsafe impl Send for RecordingSession {}
unsafe impl Sync for RecordingSession {}

type WavW = Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>;

fn make_writer(path: &PathBuf, channels: u16, sample_rate: u32) -> WavW {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let writer = WavWriter::create(path, spec).expect("create wav");
    Arc::new(Mutex::new(Some(writer)))
}

fn build_input_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    writer: WavW,
) -> Stream {
    let err_fn = |err| eprintln!("stream error: {err}");
    match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| {
                    if let Some(w) = writer.lock().unwrap().as_mut() {
                        for &s in data {
                            let v = (s * i16::MAX as f32) as i16;
                            let _ = w.write_sample(v);
                        }
                    }
                },
                err_fn,
                None,
            )
            .expect("build f32 stream"),
        SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| {
                    if let Some(w) = writer.lock().unwrap().as_mut() {
                        for &s in data {
                            let _ = w.write_sample(s);
                        }
                    }
                },
                err_fn,
                None,
            )
            .expect("build i16 stream"),
        _ => panic!("unsupported sample format"),
    }
}

pub fn start(out_dir: PathBuf) -> Result<RecordingSession, String> {
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mic_path = out_dir.join(format!("mic_{ts}.wav"));
    let sys_path = out_dir.join(format!("system_{ts}.wav"));

    let host = cpal::default_host();

    // ---- Mic ----
    let mic_dev = host.default_input_device().ok_or("no input device")?;
    let mic_cfg = mic_dev.default_input_config().map_err(|e| e.to_string())?;
    let mic_writer = make_writer(&mic_path, mic_cfg.channels(), mic_cfg.sample_rate().0);
    let mic_stream = build_input_stream(&mic_dev, &mic_cfg.clone().into(), mic_cfg.sample_format(), mic_writer);
    mic_stream.play().map_err(|e| e.to_string())?;

    // ---- System (loopback) ----
    // On Windows, cpal exposes the default output device and supports loopback via the default host.
    // For cross-platform: we attempt the default output device as input (works on Windows WASAPI).
    let sys_dev = host
        .default_output_device()
        .ok_or("no output device for loopback")?;
    let sys_cfg = sys_dev
        .default_output_config()
        .map_err(|e| e.to_string())?;
    let sys_writer = make_writer(&sys_path, sys_cfg.channels(), sys_cfg.sample_rate().0);
    let sys_stream = build_input_stream(&sys_dev, &sys_cfg.clone().into(), sys_cfg.sample_format(), sys_writer);
    sys_stream.play().map_err(|e| e.to_string())?;

    Ok(RecordingSession {
        mic_path,
        sys_path,
        _mic_stream: mic_stream,
        _sys_stream: sys_stream,
    })
}
