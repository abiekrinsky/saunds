use anyhow::{Result, Context};
use dasp::Signal;
use minimp3::{Decoder, Frame};
use num_complex::Complex;
use realfft::RealFftPlanner;
use std::{fs::File, io::BufReader, path::Path};
use tracing::{info, warn};
use hound;

pub struct AudioProcessor {
    sample_rate: u32,
    channels: u32,
}

impl AudioProcessor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            sample_rate: 44100,  // Default sample rate
            channels: 2,         // Default stereo
        })
    }

    pub fn load_audio<P: AsRef<Path>>(&self, path: P) -> Result<Vec<f32>> {
        info!("Loading audio file: {:?}", path.as_ref());
        
        let mut decoder = Decoder::new(BufReader::new(File::open(&path)?));
        let mut samples = Vec::new();
        
        let mut frame_count = 0;
        while let Ok(Frame { data, .. }) = decoder.next_frame() {
            frame_count += 1;
            info!("Processing frame {}", frame_count);
            
            // Convert i16 samples to f32 and normalize to [-1.0, 1.0]
            samples.extend(data.iter().map(|&s| s as f32 / 32768.0));
        }

        info!("Loaded {} samples from {} frames", samples.len(), frame_count);
        Ok(samples)
    }

    pub fn save_audio<P: AsRef<Path>>(&self, path: P, samples: &[f32]) -> Result<()> {
        info!("Saving audio file: {:?}", path.as_ref());
        
        let spec = hound::WavSpec {
            channels: self.channels as u16,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        
        let mut writer = hound::WavWriter::create(path, spec)
            .with_context(|| "Failed to create WAV writer")?;
        
        for &sample in samples {
            writer.write_sample(sample)
                .with_context(|| "Failed to write sample")?;
        }
        
        writer.finalize()
            .with_context(|| "Failed to finalize WAV file")?;
            
        info!("Successfully wrote {} samples", samples.len());
        Ok(())
    }

    pub fn separate_frequencies(&self, samples: &[f32], low_cutoff: f32, high_cutoff: f32) -> Result<(Vec<f32>, Vec<f32>)> {
        info!("Separating frequencies with cutoffs: low={}, high={}", low_cutoff, high_cutoff);
        
        // Convert cutoff frequencies to FFT bin indices
        let window_size = 2048;
        let overlap = window_size / 2;
        let freq_per_bin = self.sample_rate as f32 / window_size as f32;
        let low_bin = (low_cutoff / freq_per_bin) as usize;
        let high_bin = (high_cutoff / freq_per_bin) as usize;
        
        info!("FFT parameters: window_size={}, overlap={}, bins: low={}, high={}", 
             window_size, overlap, low_bin, high_bin);
        
        // Create FFT planner
        let mut planner = RealFftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);
        let ifft = planner.plan_fft_inverse(window_size);
        
        // Process audio in overlapping windows
        let mut low_freq = vec![0.0; samples.len()];
        let mut high_freq = vec![0.0; samples.len()];
        let mut window = vec![0.0; window_size];
        let mut spectrum = vec![Complex::new(0.0, 0.0); window_size / 2 + 1];
        
        // Hann window function for smooth transitions
        let window_func: Vec<f32> = (0..window_size)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / window_size as f32).cos()))
            .collect();
        
        let total_windows = (samples.len() as f32 / overlap as f32).ceil() as usize;
        let mut processed_windows = 0;
        
        for chunk_start in (0..samples.len()).step_by(overlap) {
            processed_windows += 1;
            if processed_windows % 100 == 0 {
                info!("Processing window {}/{}", processed_windows, total_windows);
            }
            
            // Fill window with samples
            window.fill(0.0);
            for i in 0..window_size {
                if chunk_start + i < samples.len() {
                    window[i] = samples[chunk_start + i] * window_func[i];
                }
            }
            
            // Forward FFT
            fft.process(&mut window, &mut spectrum)
                .with_context(|| format!("Failed to perform forward FFT on window {}", processed_windows))?;
            
            // Separate frequencies
            let mut low_spectrum = spectrum.clone();
            let mut high_spectrum = spectrum.clone();
            
            // Apply frequency masks
            for i in 0..spectrum.len() {
                if i < low_bin {
                    high_spectrum[i] = Complex::new(0.0, 0.0);
                } else if i > high_bin {
                    low_spectrum[i] = Complex::new(0.0, 0.0);
                }
            }
            
            // Inverse FFT for both frequency ranges
            let mut low_window = vec![0.0; window_size];
            let mut high_window = vec![0.0; window_size];
            
            ifft.process(&mut low_spectrum, &mut low_window)
                .with_context(|| format!("Failed to perform inverse FFT (low) on window {}", processed_windows))?;
            ifft.process(&mut high_spectrum, &mut high_window)
                .with_context(|| format!("Failed to perform inverse FFT (high) on window {}", processed_windows))?;
            
            // Overlap-add to output
            for i in 0..window_size {
                if chunk_start + i < samples.len() {
                    low_freq[chunk_start + i] += low_window[i] * window_func[i] / window_size as f32;
                    high_freq[chunk_start + i] += high_window[i] * window_func[i] / window_size as f32;
                }
            }
        }
        
        info!("Frequency separation complete. Processed {} windows", processed_windows);
        Ok((low_freq, high_freq))
    }
} 