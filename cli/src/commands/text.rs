use crate::config::NetworkConfig;
use crate::output::{print_human, with_spinner, OutputFormat};
use crate::signer::SignerProfile;
use anyhow::Context;
use xlm_ns_sdk::client::XlmNsClient;
use xlm_ns_sdk::types::TextRecordUpdate;

pub async fn run_get(
    config: NetworkConfig,
    output: OutputFormat,
    name: &str,
    key: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let record = with_spinner(
        format!("Fetching text record {name}:{key}"),
        output,
        client.get_text_record(name, key),
    )
    .await
    .context("Failed to fetch text record")?;

    if let Some(val) = record.value {
        print_human(&format!("{}: {} = \"{}\"", name, key, val));
    } else {
        print_human(&format!("{}: {} = [NOT SET]", name, key));
    }

    Ok(())
}

pub async fn run_set(
    config: NetworkConfig,
    output: OutputFormat,
    name: &str,
    key: &str,
    value: Option<String>,
    signer: Option<SignerProfile>,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    if let Some(ref s) = signer {
        print_human(&format!("  Signer: {}", s.describe()));
    }

    let submission = with_spinner(
        format!("Submitting text record update for {name}:{key}"),
        output,
        client.set_text_record(TextRecordUpdate {
            name: name.into(),
            key: key.into(),
            value,
            signer: signer.as_ref().map(|s| s.name.clone()),
        }),
    )
    .await
    .context("Failed to update text record")?;

    print_human(&format!(
        "SUCCESS: text record update submitted\n  Status: {}\n  Transaction Hash: {}",
        submission.status, submission.tx_hash
    ));

    Ok(())
}
