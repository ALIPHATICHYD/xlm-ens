use crate::config::NetworkConfig;
use crate::output::{print_human, with_spinner, OutputFormat};
use anyhow::{anyhow, Context};
use xlm_ns_sdk::client::XlmNsClient;
use xlm_ns_sdk::types::{BuildMessageRequest, RegisterChainRequest};

pub async fn run_register_chain(
    config: NetworkConfig,
    output: OutputFormat,
    chain: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    with_spinner(
        format!("Submitting bridge route registration for {chain}"),
        output,
        client.register_chain(RegisterChainRequest {
            chain: chain.into(),
        }),
    )
    .await
    .context("Failed to register chain")?;

    print_human(&format!("SUCCESS: registered bridge route for chain {}", chain));
    Ok(())
}

pub async fn run_inspect_route(config: NetworkConfig, chain: &str) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let route = client
        .get_route(chain)
        .await
        .context("Failed to inspect route")?
        .ok_or_else(|| anyhow!("No route found for chain '{}'", chain))?;

    print_human(&format!(
        "Bridge route for chain '{}':\n  Chain: {}\n  Gateway: {}\n  Resolver: {}",
        chain, route.destination_chain, route.gateway, route.destination_resolver
    ));

    Ok(())
}

pub async fn run_generate_payload(
    config: NetworkConfig,
    output: OutputFormat,
    name: &str,
    chain: &str,
) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let payload = client
        .build_message(BuildMessageRequest {
            name: name.into(),
            chain: chain.into(),
        })
        .await
        .context("Failed to generate payload")?;

    if output == OutputFormat::Human {
        print_human(&format!(
            "Generated payload for '{}' on chain '{}':\n{}",
            name, chain, payload
        ));
    } else {
        println!("{}", payload);
    }

    Ok(())
}
