use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error, Level};

mod audio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input audio file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory path
    #[arg(short, long)]
    output: PathBuf,

    /// Low frequency cutoff (Hz)
    #[arg(long, default_value = "200")]
    low_cutoff: f32,

    /// High frequency cutoff (Hz)
    #[arg(long, default_value = "2000")]
    high_cutoff: f32,
}

fn main() -> Result<()> {
    // Initialize basic logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let cli = Args::parse();

    info!("Starting audio processing...");
    info!("Input file: {}", cli.input.display());
    info!("Output directory: {}", cli.output.display());
    info!("Frequency cutoffs: {} Hz - {} Hz", cli.low_cutoff, cli.high_cutoff);

    // Verify input file exists
    if !cli.input.exists() {
        error!("Input file does not exist: {}", cli.input.display());
        return Ok(());
    }

    // Create output directory if it does not exist
    if !cli.output.exists() {
        info!("Creating output directory: {}", cli.output.display());
        std::fs::create_dir_all(&cli.output)?;
    }

    // Initialize audio processor
    let processor = audio::AudioProcessor::new()?;

    // Load audio file
    info!("Loading audio file...");
    let samples = processor.load_audio(&cli.input)?;
    info!("Loaded {} samples", samples.len());

    // Separate frequencies
    info!("Separating frequencies...");
    let (low_freq, high_freq) = match processor.separate_frequencies(
        &samples,
        cli.low_cutoff,
        cli.high_cutoff,
    ) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to separate frequencies: {}", e);
            return Err(e);
        }
    };

    // Save separated audio files
    let low_freq_path = cli.output.join("low_freq.wav");
    let high_freq_path = cli.output.join("high_freq.wav");

    info!("Saving low frequency audio to: {}", low_freq_path.display());
    processor.save_audio(&low_freq_path, &low_freq)?;

    info!("Saving high frequency audio to: {}", high_freq_path.display());
    processor.save_audio(&high_freq_path, &high_freq)?;

    info!("Audio processing completed successfully!");
    Ok(())
}