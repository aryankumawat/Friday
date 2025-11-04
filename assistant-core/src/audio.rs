use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Sample, SampleFormat, Stream, StreamConfig, SizedSample, DevicesError};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("No audio input device found")]
    NoInputDevice,
    #[error("Failed to get device name: {0}")]
    DeviceName(#[from] cpal::DeviceNameError),
    #[error("Failed to get supported configs: {0}")]
    SupportedConfigs(#[from] cpal::SupportedStreamConfigsError),
    #[error("Failed to build stream: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error("Failed to play stream: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error("Failed to enumerate devices: {0}")]
    Devices(#[from] DevicesError),
    #[error("Unsupported sample format: {0:?}")]
    UnsupportedFormat(SampleFormat),
}

pub type AudioResult<T> = Result<T, AudioError>;

/// Audio sample data with metadata
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp: std::time::Instant,
}

impl AudioChunk {
    pub fn new(data: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self {
            data,
            sample_rate,
            channels,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Convert to mono if stereo
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            self.data.clone()
        } else {
            // Simple stereo to mono conversion (average channels)
            self.data
                .chunks_exact(self.channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / self.channels as f32)
                .collect()
        }
    }

    /// Resample to target sample rate (simple linear interpolation)
    pub fn resample(&self, target_rate: u32) -> Vec<f32> {
        if self.sample_rate == target_rate {
            return self.to_mono();
        }

        let mono_data = self.to_mono();
        let ratio = self.sample_rate as f64 / target_rate as f64;
        let target_len = (mono_data.len() as f64 / ratio) as usize;
        
        let mut resampled = Vec::with_capacity(target_len);
        
        for i in 0..target_len {
            let src_index = (i as f64 * ratio) as usize;
            if src_index < mono_data.len() {
                resampled.push(mono_data[src_index]);
            } else {
                resampled.push(0.0);
            }
        }
        
        resampled
    }
}

/// Audio capture configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
    pub device_name: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000, // Common for speech recognition
            channels: 1,        // Mono
            buffer_size: 1024,  // ~64ms at 16kHz
            device_name: None,  // Use default device
        }
    }
}

/// Real-time audio capture from microphone
pub struct AudioCapture {
    _stream: Stream,
    receiver: Receiver<AudioChunk>,
    config: AudioConfig,
}

// Make AudioCapture Send by ensuring Stream is handled properly
unsafe impl Send for AudioCapture {}

impl AudioCapture {
    /// Create new audio capture with default device
    pub fn new(config: AudioConfig) -> AudioResult<Self> {
        let host = cpal::default_host();
        let device = if let Some(name) = &config.device_name {
            Self::find_device_by_name(&host, name)?
        } else {
            host.default_input_device()
                .ok_or(AudioError::NoInputDevice)?
        };

        Self::new_with_device(device, config)
    }

    /// Create audio capture with specific device
    pub fn new_with_device(device: Device, config: AudioConfig) -> AudioResult<Self> {
        let device_name = device.name()?;
        info!("Using audio device: {}", device_name);

        // Get supported configurations
        let supported_configs = device.supported_input_configs()?;
        let supported_config = supported_configs
            .filter(|c| c.channels() <= config.channels)
            .min_by_key(|c| {
                // Prefer configs closer to our target sample rate
                (c.min_sample_rate().0 as i32 - config.sample_rate as i32).abs()
            })
            .ok_or(AudioError::NoInputDevice)?;

        debug!("Selected config: {:?}", supported_config);

        // Build stream configuration
        let stream_config = StreamConfig {
            channels: config.channels,
            sample_rate: cpal::SampleRate(config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size as u32),
        };

        let (sender, receiver) = mpsc::channel();
        let buffer_size = config.buffer_size;
        let sample_rate = config.sample_rate;
        let channels = config.channels;

        // Build the audio stream based on sample format
        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => Self::build_stream::<f32>(&device, &stream_config, sender, buffer_size, sample_rate, channels)?,
            SampleFormat::I16 => Self::build_stream::<i16>(&device, &stream_config, sender, buffer_size, sample_rate, channels)?,
            SampleFormat::U16 => Self::build_stream::<u16>(&device, &stream_config, sender, buffer_size, sample_rate, channels)?,
            format => return Err(AudioError::UnsupportedFormat(format)),
        };

        Ok(Self {
            _stream: stream,
            receiver,
            config,
        })
    }

    /// Build audio stream for specific sample type
    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        sender: Sender<AudioChunk>,
        buffer_size: usize,
        sample_rate: u32,
        channels: u16,
    ) -> AudioResult<Stream>
    where
        T: Sample + SizedSample + Send + 'static,
        f32: From<T>,
    {
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(buffer_size)));
        let buffer_clone = buffer.clone();

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap();
                
                // Convert samples to f32 and add to buffer
                for &sample in data {
                    buf.push(f32::from(sample));
                    
                    // Send chunk when buffer is full
                    if buf.len() >= buffer_size {
                        let chunk = AudioChunk::new(
                            buf.drain(..).collect(),
                            sample_rate,
                            channels,
                        );
                        
                        if let Err(e) = sender.send(chunk) {
                            warn!("Failed to send audio chunk: {}", e);
                        }
                    }
                }
            },
            |err| error!("Audio stream error: {}", err),
            None,
        )?;

        Ok(stream)
    }

    /// Find device by name
    fn find_device_by_name(host: &Host, name: &str) -> AudioResult<Device> {
        for device in host.input_devices()? {
            if device.name()? == name {
                return Ok(device);
            }
        }
        Err(AudioError::NoInputDevice)
    }

    /// Start capturing audio
    pub fn start(&self) -> AudioResult<()> {
        self._stream.play()?;
        info!("Audio capture started");
        Ok(())
    }

    /// Get the next audio chunk (blocking)
    pub fn next_chunk(&self) -> Option<AudioChunk> {
        self.receiver.recv().ok()
    }

    /// Try to get the next audio chunk (non-blocking)
    pub fn try_next_chunk(&self) -> Option<AudioChunk> {
        self.receiver.try_recv().ok()
    }

    /// Get audio configuration
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }
}

/// List available audio input devices
pub fn list_input_devices() -> AudioResult<Vec<String>> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    
    for device in host.input_devices()? {
        devices.push(device.name()?);
    }
    
    Ok(devices)
}

/// Get default input device info
pub fn default_input_device_info() -> AudioResult<String> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or(AudioError::NoInputDevice)?;
    
    Ok(device.name()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_chunk_mono_conversion() {
        // Test stereo to mono conversion
        let stereo_data = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]; // 3 stereo frames
        let chunk = AudioChunk::new(stereo_data, 16000, 2);
        let mono = chunk.to_mono();
        
        assert_eq!(mono.len(), 3);
        assert_eq!(mono[0], 0.15); // (0.1 + 0.2) / 2
        assert_eq!(mono[1], 0.35); // (0.3 + 0.4) / 2
        assert_eq!(mono[2], 0.55); // (0.5 + 0.6) / 2
    }

    #[test]
    fn test_audio_chunk_resampling() {
        // Test simple resampling
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let chunk = AudioChunk::new(data, 8000, 1);
        let resampled = chunk.resample(4000); // Downsample by 2x
        
        assert_eq!(resampled.len(), 2);
        assert_eq!(resampled[0], 1.0);
        assert_eq!(resampled[1], 3.0);
    }

    #[tokio::test]
    async fn test_list_devices() {
        // This test requires audio devices to be available
        match list_input_devices() {
            Ok(devices) => {
                println!("Available devices: {:?}", devices);
                // Should have at least one device on most systems
            }
            Err(e) => {
                println!("No audio devices available: {}", e);
            }
        }
    }
}