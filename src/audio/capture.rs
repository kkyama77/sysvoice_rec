use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;

/// System audio loopback capture.
/// Uses WASAPI loopback on Windows (output device as input).
/// Stream stops automatically when dropped.
pub struct AudioCapture {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioCapture {
    /// Start loopback capture, sending f32 samples to sender.
    pub fn start(sender: Sender<Vec<f32>>) -> Result<Self> {
        let host = cpal::default_host();

        #[cfg(windows)]
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device found"))?;

        #[cfg(not(windows))]
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

        #[cfg(windows)]
        log::info!("capture device (loopback output): {:?}", device.name().ok());
        #[cfg(not(windows))]
        log::info!("capture device (default input): {:?}", device.name().ok());

        #[cfg(windows)]
        let supported = device
            .default_output_config()
            .map_err(|e| anyhow::anyhow!("Failed to get output config: {e}"))?;

        #[cfg(not(windows))]
        let supported = device
            .default_input_config()
            .map_err(|e| anyhow::anyhow!("Failed to get input config: {e}"))?;

        let sample_rate = supported.sample_rate().0;
        let channels    = supported.channels();
        let fmt         = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        log::info!("stream: {}Hz {}ch {:?}", sample_rate, channels, fmt);

        // clone sender for each format arm
        let s_f32 = sender.clone();
        let s_i16 = sender.clone();
        let s_i32 = sender;

        let stream = match fmt {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let _ = s_f32.try_send(data.to_vec());
                },
                |e| log::error!("stream error: {e}"),
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let v: Vec<f32> = data
                        .iter()
                        .map(|&s| s as f32 / i16::MAX as f32)
                        .collect();
                    let _ = s_i16.try_send(v);
                },
                |e| log::error!("stream error: {e}"),
                None,
            )?,
            cpal::SampleFormat::I32 => device.build_input_stream(
                &config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let v: Vec<f32> = data
                        .iter()
                        .map(|&s| s as f32 / i32::MAX as f32)
                        .collect();
                    let _ = s_i32.try_send(v);
                },
                |e| log::error!("stream error: {e}"),
                None,
            )?,
            cpal::SampleFormat::U8 => device.build_input_stream(
                &config,
                move |data: &[u8], _: &cpal::InputCallbackInfo| {
                    let v: Vec<f32> = data
                        .iter()
                        .map(|&s| (s as f32 - 128.0) / 128.0)
                        .collect();
                    let _ = s_f32.try_send(v);
                },
                |e| log::error!("stream error: {e}"),
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let v: Vec<f32> = data
                        .iter()
                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                        .collect();
                    let _ = s_i16.try_send(v);
                },
                |e| log::error!("stream error: {e}"),
                None,
            )?,
            other => {
                anyhow::bail!("Unsupported sample format: {other:?}");
            }
        };

        stream.play()?;
        #[cfg(windows)]
        log::info!("loopback capture started");
        #[cfg(not(windows))]
        log::info!("input capture started");

        Ok(Self { _stream: stream, sample_rate, channels })
    }
}
