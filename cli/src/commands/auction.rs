use crate::config::NetworkConfig;
use crate::output::{print_human, with_spinner, OutputFormat};
use crate::signer::SignerProfile;
use anyhow::{anyhow, Context};
use xlm_ns_sdk::client::XlmNsClient;
use xlm_ns_sdk::types::{AuctionCreateRequest, BidRequest};

pub async fn run_create(
    config: NetworkConfig,
    output: OutputFormat,
    name: &str,
    reserve: u64,
    duration: u64,
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

    print_human(&format!("Creating auction for {name}..."));
    if let Some(ref s) = signer {
        print_human(&format!("  Signer: {}", s.describe()));
    }
    let treasury = signer
        .as_ref()
        .map(|s| s.public_address.clone())
        .unwrap_or_else(|| format!("G{}", "A".repeat(55)));

    let submission = with_spinner(
        format!("Submitting auction creation for {name}"),
        output,
        client.create_auction(AuctionCreateRequest {
            name: name.into(),
            asset: "XLM".to_string(),
            treasury,
            reserve_price: reserve,
            duration_seconds: duration,
            signer: signer.as_ref().map(|s| s.name.clone()),
        }),
    )
    .await
    .context("Failed to create auction")?;

    print_human(&format!(
        "SUCCESS: auction created for {name}\n  Reserve: {reserve} XLM\n  Duration: {duration}s\n  Transaction Hash: {}",
        submission.tx_hash
    ));

    Ok(())
}

pub async fn run_bid(
    config: NetworkConfig,
    output: OutputFormat,
    name: &str,
    amount: u64,
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

    print_human(&format!("Placing bid of {amount} XLM on {name}..."));
    if let Some(ref s) = signer {
        print_human(&format!("  Signer: {}", s.describe()));
    }

    let submission = with_spinner(
        format!("Submitting bid for {name}"),
        output,
        client.bid_auction(BidRequest {
            name: name.into(),
            amount,
            signer: signer.as_ref().map(|s| s.name.clone()),
        }),
    )
    .await
    .context("Failed to place bid")?;

    print_human(&format!(
        "SUCCESS: bid placed on {name}\n  Transaction Hash: {}",
        submission.tx_hash
    ));

    Ok(())
}

pub async fn run_inspect(config: NetworkConfig, name: &str) -> anyhow::Result<()> {
    let client = XlmNsClient::new(
        config.rpc_url,
        Some(config.network_passphrase),
        config.registry_contract_id.clone(),
        config.subdomain_contract_id.clone(),
        config.bridge_contract_id.clone(),
        config.auction_contract_id.clone(),
    );

    let auction = client
        .get_auction(name)
        .await
        .context("Failed to fetch auction state")?
        .ok_or_else(|| anyhow!("No active auction found for '{}'", name))?;

    print_human(&format!(
        "Auction for {}:\n  Status: {}\n  Owner: {}\n  Reserve Price: {} XLM\n  Highest Bid: {} XLM",
        auction.name, auction.status, auction.owner, auction.reserve_price, auction.highest_bid
    ));
    if let Some(bidder) = auction.highest_bidder {
        print_human(&format!("  Highest Bidder: {}", bidder));
    }
    print_human(&format!("  Ends at: {}", auction.ends_at));

    Ok(())
}

pub async fn run_settle(
    config: NetworkConfig,
    output: OutputFormat,
    name: &str,
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

    print_human(&format!("Settling auction for {name}..."));
    if let Some(ref s) = signer {
        print_human(&format!("  Signer: {}", s.describe()));
    }

    let submission = with_spinner(
        format!("Submitting settlement for {name}"),
        output,
        client.settle_auction(name, signer.as_ref().map(|s| s.name.clone())),
    )
    .await
    .context("Failed to settle auction")?;

    print_human(&format!(
        "SUCCESS: auction settled for {name}\n  Transaction Hash: {}",
        submission.tx_hash
    ));

    Ok(())
}
//
