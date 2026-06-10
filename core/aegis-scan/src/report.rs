//! 스캔 결과 출력

use crate::orchestrator::ScanResults;
use aegis_common::types::ThreatVerdict;

pub fn print_summary(results: &ScanResults) {
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║         AegisRescue — Scan Report                ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║ Target  : {:<39}║", results.target.display());
    println!("║ Scan ID : {:<39}║", &results.scan_id.to_string()[..36]);
    println!("╠══════════════════════════════════════════════════╣");
    println!("║ Files scanned   : {:>8}                     ║", results.scanned_files);
    println!("║ Malicious       : {:>8}                     ║", results.malicious_count);
    println!("║ Suspicious      : {:>8}                     ║", results.suspicious_count);
    println!("║ Clean           : {:>8}                     ║", results.clean_count);
    println!("║ Skipped         : {:>8}                     ║", results.skipped_count);
    println!("╚══════════════════════════════════════════════════╝");

    if !results.threats.is_empty() {
        println!();
        println!("Threats detected:");
        for threat in &results.threats {
            let verdict_icon = match threat.verdict {
                ThreatVerdict::Malicious  => "🔴 MALICIOUS",
                ThreatVerdict::Suspicious => "🟡 SUSPICIOUS",
                _ => "⚪ CLEAN",
            };
            println!(
                "  {} {:.2} | {} | {}",
                verdict_icon,
                threat.risk_score,
                threat.malware_family.as_deref().unwrap_or("Unknown"),
                threat.path.display()
            );

            for det in &threat.detections {
                println!(
                    "    ├─ [{:?}] {:.0}% confidence — {}",
                    det.engine,
                    det.confidence * 100.0,
                    det.signature.as_deref().unwrap_or("-"),
                );
            }
        }
        println!();
    }

    let duration = (results.completed_at - results.started_at).num_seconds();
    println!("Scan completed in {}s", duration);

    if results.malicious_count > 0 {
        println!();
        println!("⚠ {} malicious file(s) found. Run with --quarantine to isolate.", results.malicious_count);
    } else if results.suspicious_count > 0 {
        println!("⚠ {} suspicious file(s) found. Consider sandbox analysis.", results.suspicious_count);
    } else {
        println!("✓ No threats detected.");
    }
}
