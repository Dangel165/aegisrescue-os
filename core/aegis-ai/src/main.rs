//! aegis-ai CLI

mod features;
mod model;
mod result;

use features::FeatureVector;
use model::AiClassifier;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aegis-ai", version, about = "AI-based malware classifier")]
struct Cli {
    /// 분석할 파일
    file: PathBuf,
    /// AI 모델 디렉터리
    #[arg(long, default_value = "/usr/share/aegis/models")]
    model_dir: PathBuf,
    /// JSON 출력
    #[arg(long)]
    json: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let features = FeatureVector::extract(&cli.file)?;
    let classifier = AiClassifier::new(&cli.model_dir);
    let result = classifier.classify(&features);

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("File:       {}", cli.file.display());
        println!("Class:      {:?}", result.class);
        println!("Confidence: {:.1}%", result.confidence * 100.0);
        println!("Entropy:    {:.2} bits/byte", features.entropy * 8.0);
        if !result.key_features.is_empty() {
            println!("Reasons:");
            for r in &result.key_features {
                println!("  - {}", r);
            }
        }
        println!();
        println!("Class probabilities:");
        let mut probs = result.probabilities.clone();
        probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (name, prob) in &probs {
            println!("  {:15} {:.1}%", name, prob * 100.0);
        }
    }

    if result.is_malicious() {
        std::process::exit(1);
    }

    Ok(())
}
