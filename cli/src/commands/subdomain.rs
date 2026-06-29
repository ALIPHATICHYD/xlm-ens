use crate::config::NetworkConfig;
use crate::output::{print_human, with_spinner, OutputFormat};
use anyhow::Context;
use xlm_ns_sdk::client::XlmNsClient;
use xlm_ns_sdk::types::{
    AddControllerRequest, CreateSubdomainRequest, RegisterParentRequest, TransferSubdomainRequest,
};

pub async fn run_register_parent(
    config: NetworkConfig,
    output: OutputFormat,
    parent: &str,
    owner: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let submission = with_spinner(
        format!("Submitting parent registration for {parent}"),
        output,
        client.register_parent(
            RegisterParentRequest {
                parent: parent.into(),
                owner: owner.into(),
            },
            false,
        ),
    )
    .await
    .context("Failed to register parent domain")?;

    print_human(&format!(
        "SUCCESS: registered parent domain {parent} with owner {owner}\n  Transaction Hash: {}",
        submission.tx_hash
    ));
    Ok(())
}

pub async fn run_add_controller(
    config: NetworkConfig,
    output: OutputFormat,
    parent: &str,
    controller: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let submission = with_spinner(
        format!("Submitting controller update for {parent}"),
        output,
        client.add_controller(
            AddControllerRequest {
                parent: parent.into(),
                controller: controller.into(),
            },
            false,
        ),
    )
    .await
    .context("Failed to add controller")?;

    print_human(&format!(
        "SUCCESS: added controller {controller} to parent domain {parent}\n  Transaction Hash: {}",
        submission.tx_hash
    ));
    Ok(())
}

pub async fn run_create_subdomain(
    config: NetworkConfig,
    output: OutputFormat,
    label: &str,
    parent: &str,
    owner: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let submission = with_spinner(
        format!("Submitting subdomain creation for {label}.{parent}"),
        output,
        client.create_subdomain(
            CreateSubdomainRequest {
                label: label.into(),
                parent: parent.into(),
                owner: owner.into(),
            },
            false,
        ),
    )
    .await
    .context("Failed to create subdomain")?;

    let fqdn = format!("{label}.{parent}");
    print_human(&format!(
        "SUCCESS: created subdomain {fqdn} with owner {owner}\n  Transaction Hash: {}",
        submission.tx_hash
    ));
    Ok(())
}

pub async fn run_transfer_subdomain(
    config: NetworkConfig,
    output: OutputFormat,
    fqdn: &str,
    new_owner: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let submission = with_spinner(
        format!("Submitting subdomain transfer for {fqdn}"),
        output,
        client.transfer_subdomain(
            TransferSubdomainRequest {
                fqdn: fqdn.into(),
                new_owner: new_owner.into(),
            },
            false,
        ),
    )
    .await
    .context("Failed to transfer subdomain")?;

    print_human(&format!(
        "SUCCESS: transferred subdomain {fqdn} to new owner {new_owner}\n  Transaction Hash: {}",
        submission.tx_hash
    ));
    Ok(())
}
