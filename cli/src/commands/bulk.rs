use crate::config::NetworkConfig;
use crate::output::{print_human, progress_bar, with_spinner, OutputFormat};
use anyhow::Context;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CsvRecord {
    name: String,
    duration: u64,
    owner: String,
}

#[derive(Debug, Deserialize)]
struct JsonRecord {
    name: String,
    duration: u64,
    owner: String,
}

pub async fn run_bulk_register(
    config: NetworkConfig,
    file: &PathBuf,
    dry_run: bool,
) -> anyhow::Result<()> {
    let file_extension = file.extension().and_then(|s| s.to_str()).unwrap_or("");

    let records: Vec<JsonRecord> = match file_extension {
        "csv" => {
            let file = File::open(file).context("Failed to open file")?;
            let mut rdr = csv::Reader::from_reader(BufReader::new(file));
            let mut records = Vec::new();
            for result in rdr.deserialize() {
                let record: CsvRecord = result.context("Failed to deserialize CSV record")?;
                records.push(JsonRecord {
                    name: record.name,
                    duration: record.duration,
                    owner: record.owner,
                });
            }
            records
        }
        "json" => {
            let file = File::open(file).context("Failed to open file")?;
            let reader = BufReader::new(file);
            let records: Vec<JsonRecord> =
                serde_json::from_reader(reader).context("Failed to deserialize JSON")?;
            records
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported file format. Please use a .csv or .json file."
            ));
        }
    };

    if dry_run {
        print_human("Dry run: The following names would be registered:");
        for record in records {
            print_human(&format!(
                "  - Name: {}, Duration: {}, Owner: {}",
                record.name, record.duration, record.owner
            ));
        }
    } else {
        let registrar_id = config
            .registrar_contract_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Registrar contract ID not configured"))?;

        let client = xlm_ns_sdk::client::XlmNsClient::new(
            config.rpc_url.clone(),
            Some(config.network_passphrase.clone()),
            config.registry_contract_id.clone(),
            config.subdomain_contract_id.clone(),
            config.bridge_contract_id.clone(),
            config.auction_contract_id.clone(),
        )
        .with_registrar(registrar_id.clone());

        let progress = progress_bar(records.len() as u64, "Bulk register", OutputFormat::Human);
        for record in records {
            print_human(&format!("Registering {}...", record.name));
            match with_spinner(
                format!("Submitting registration for {}", record.name),
                OutputFormat::Human,
                client.register(xlm_ns_sdk::types::RegistrationRequest {
                    label: record.name.clone(),
                    owner: record.owner.clone(),
                    duration_years: record.duration,
                    signer: None,
                }),
            )
            .await
            {
                Ok(receipt) => {
                    print_human(&format!(
                        "  - SUCCESS: registered {} to {}\n    Fee paid: {} XLM\n    Expires at: {}\n    Status: {}\n    Transaction Hash: {}",
                        receipt.name, receipt.owner, receipt.fee_paid, receipt.expires_at, receipt.submission.status, receipt.submission.tx_hash
                    ));
                }
                Err(e) => {
                    print_human(&format!("  - ERROR: Failed to register {}: {}", record.name, e));
                }
            }
            if let Some(ref bar) = progress {
                bar.inc(1);
            }
        }
        if let Some(bar) = progress {
            bar.finish_with_message("Bulk register complete");
        }
    }

    Ok(())
}

pub async fn run_bulk_renew(
    config: NetworkConfig,
    file: &PathBuf,
    dry_run: bool,
) -> anyhow::Result<()> {
    let file_extension = file.extension().and_then(|s| s.to_str()).unwrap_or("");

    let records: Vec<RenewJsonRecord> = match file_extension {
        "csv" => {
            let file = File::open(file).context("Failed to open file")?;
            let mut rdr = csv::Reader::from_reader(BufReader::new(file));
            let mut records = Vec::new();
            for result in rdr.deserialize() {
                let record: RenewCsvRecord = result.context("Failed to deserialize CSV record")?;
                records.push(RenewJsonRecord {
                    name: record.name,
                    duration: record.duration,
                });
            }
            records
        }
        "json" => {
            let file = File::open(file).context("Failed to open file")?;
            let reader = BufReader::new(file);
            let records: Vec<RenewJsonRecord> =
                serde_json::from_reader(reader).context("Failed to deserialize JSON")?;
            records
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported file format. Please use a .csv or .json file."
            ));
        }
    };

    if dry_run {
        print_human("Dry run: The following names would be renewed:");
        for record in records {
            print_human(&format!("  - Name: {}, Duration: {}", record.name, record.duration));
        }
    } else {
        let registrar_id = config
            .registrar_contract_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Registrar contract ID not configured"))?;

        let client = xlm_ns_sdk::client::XlmNsClient::new(
            config.rpc_url.clone(),
            Some(config.network_passphrase.clone()),
            config.registry_contract_id.clone(),
            config.subdomain_contract_id.clone(),
            config.bridge_contract_id.clone(),
            config.auction_contract_id.clone(),
        )
        .with_registrar(registrar_id.clone());

        let progress = progress_bar(records.len() as u64, "Bulk renew", OutputFormat::Human);
        for record in records {
            print_human(&format!("Renewing {}...", record.name));
            match with_spinner(
                format!("Submitting renewal for {}", record.name),
                OutputFormat::Human,
                client.renew(xlm_ns_sdk::types::RenewRequest {
                    label: record.name.clone(),
                    duration_years: record.duration,
                    signer: None,
                }),
            )
            .await
            {
                Ok(receipt) => {
                    print_human(&format!(
                        "  - SUCCESS: renewed {}\n    Fee paid: {} XLM\n    Expires at: {}\n    Status: {}\n    Transaction Hash: {}",
                        receipt.name, receipt.fee_paid, receipt.expires_at, receipt.submission.status, receipt.submission.tx_hash
                    ));
                }
                Err(e) => {
                    print_human(&format!("  - ERROR: Failed to renew {}: {}", record.name, e));
                }
            }
            if let Some(ref bar) = progress {
                bar.inc(1);
            }
        }
        if let Some(bar) = progress {
            bar.finish_with_message("Bulk renew complete");
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct RenewCsvRecord {
    name: String,
    duration: u64,
}

#[derive(Debug, Deserialize)]
struct RenewJsonRecord {
    name: String,
    duration: u64,
}
