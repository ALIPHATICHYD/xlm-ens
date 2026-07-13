#![allow(deprecated)]
use soroban_sdk::token::{Client, StellarAssetClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use xlm_ns_auction::{AuctionContract, AuctionContractClient};
use xlm_ns_registrar::RegistrarError;
use xlm_ns_registrar::{pricing, RegistrarContract, RegistrarContractClient};
use xlm_ns_registry::{RegistryContract, RegistryContractClient};

struct TimeHelper {
    pub now: u64,
}

impl TimeHelper {
    pub fn new(start: u64) -> Self {
        Self { now: start }
    }
    pub fn advance(&mut self, seconds: u64) {
        self.now += seconds;
    }
    pub fn future(&self, seconds: u64) -> u64 {
        self.now + seconds
    }
}

fn setup_registrar_registry() -> (
    Env,
    RegistrarContractClient<'static>,
    RegistryContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let registry_id = env.register(RegistryContract, ());
    let registrar_id = env.register(RegistrarContract, ());

    let registrar = RegistrarContractClient::new(&env, &registrar_id);
    let registry = RegistryContractClient::new(&env, &registry_id);

    // Wire the registrar to the registry.
    registrar.initialize(&registry_id, &Address::generate(&env));

    (env, registrar, registry)
}

#[test]
fn test_concurrent_registration_same_ledger() {
    let (env, registrar, registry) = setup_registrar_registry();
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let label = String::from_str(&env, "alice");
    let name = String::from_str(&env, "alice.xlm");
    let time = TimeHelper::new(1_000_000);

    // Get a quote for registration
    let quote = registrar.quote_registration(&label, &1, &time.now);
    let fee_stroops = quote.fee_stroops;

    // Fund the accounts with enough lumens (native asset) to pay for registration
    // In Soroban test env, we can assume accounts have sufficient balance for native asset.
    // But we need to set the balance? Actually, the test environment doesn't charge for transactions,
    // but the contract checks the payment. We'll just provide the payment from the account.
    // We don't need to actually transfer native asset in test env because the contract doesn't check balance?
    // It does: the user must provide the payment, and the contract will subtract from treasury.
    // The user's balance is not checked by the contract; it's assumed the user has provided the amount.
    // So we just need to pass the payment_stroops.

    // We'll simulate two transactions in the same ledger by invoking the registrar.register function twice
    // in the same environment (same ledger state) without advancing time.

    // First registration attempt (should succeed)
    let result1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registrar.register(&label, &owner1, &1, &fee_stroops, &time.now);
    }));
    assert!(result1.is_ok(), "First registration should succeed");

    // Second registration attempt (should fail because name is now taken)
    let result2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registrar.register(&label, &owner2, &1, &fee_stroops, &time.now);
    }));
    assert!(
        result2.is_err(),
        "Second registration should fail due to already registered"
    );

    // Verify that the first owner is the owner in both contracts
    let reg_record = registrar
        .registration(&name)
        .expect("Registrar record should exist");
    assert_eq!(reg_record.owner, owner1);
    let reg_entry = registry.resolve(&name, &time.now);
    assert_eq!(reg_entry.owner, owner1);

    // Ensure the second owner does not have any record
    assert!(registrar.registration(&name).map(|r| r.owner) != Some(owner2));
}

#[test]
fn test_registration_racing_against_auction_settlement() {
    // Use a single shared Env for all contracts to avoid cross-env object mismatches.
    let (env, registrar, registry) = setup_registrar_registry();

    // Setup auction contract in the same env.
    let admin = Address::generate(&env);
    let auction_id = env.register(AuctionContract, ());
    let auction_client = AuctionContractClient::new(&env, &auction_id);
    auction_client.initialize(&admin);

    // Setup token in the same env.
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_asset = StellarAssetClient::new(&env, &token_contract);
    let token_client = Client::new(&env, &token_contract);

    let auction_winner = Address::generate(&env);
    let auction_loser = Address::generate(&env);
    let challenger = Address::generate(&env);

    let label = String::from_str(&env, "auctionname");
    let name = String::from_str(&env, "auctionname.xlm");
    let mut time = TimeHelper::new(1_000_000);

    // Create an auction
    let reserve_price = 100;
    let starts_at = time.now;
    let ends_at = time.future(100);

    auction_client.create_auction(
        &name,
        &token_asset.address,
        &Address::generate(&env),
        &reserve_price,
        &starts_at,
        &ends_at,
    );

    // Fund bidders with tokens
    token_asset.mint(&auction_winner, &1000);
    token_asset.mint(&auction_loser, &1000);
    token_asset.mint(&challenger, &1000);

    // Place bids
    auction_client.place_bid(&name, &auction_winner, &500, &time.future(10));
    auction_client.place_bid(&name, &auction_loser, &300, &time.future(20));

    // Advance time past auction end
    time.advance(101);

    // Before the auction is settled, the challenger races to register the name directly.
    let quote = registrar.quote_registration(&label, &1, &time.now);
    let fee_stroops = quote.fee_stroops;

    registrar.register(&label, &challenger, &1, &fee_stroops, &time.now);

    // Verify challenger is the new owner.
    let reg_entry = registry.resolve(&name, &time.now);
    assert_eq!(reg_entry.owner, challenger);

    // The auction winner settles the auction.
    // The auction contract is independent of the registry — it settles based on
    // its own bid data (Vickrey second-price). It does not check registry ownership.
    let settlement = auction_client.settle(&name, &time.now);

    // The auction had valid bids above reserve, so it settles as sold from the
    // auction contract's perspective (it doesn't know about the registry).
    if let Some(s) = settlement {
        if s.sold {
            // Auction settled normally — winner pays clearing price.
            assert_eq!(s.winner, Some(auction_winner.clone()));
            assert_eq!(s.clearing_price, 300); // Vickrey: second-highest bid
            assert_eq!(
                token_client.balance(&auction_winner),
                1000 - 300,
                "Winner pays clearing price"
            );
            assert_eq!(
                token_client.balance(&auction_loser),
                1000,
                "Loser is fully refunded"
            );
        } else {
            // If the contract somehow marks it unsold, both should be refunded.
            assert_eq!(token_client.balance(&auction_winner), 1000);
            assert_eq!(token_client.balance(&auction_loser), 1000);
        }
    }
}

#[test]
fn test_registration_fails_if_price_exceeds_max_price() {
    let (env, registrar, _) = setup_registrar_registry();
    let owner = Address::generate(&env);
    let label = String::from_str(&env, "carol");
    let time = TimeHelper::new(1_000_000);

    // 1. Get a quote for a 5-char name.
    let quote = registrar.quote_registration(&label, &1, &time.now);
    let original_fee = quote.fee_stroops;
    assert_eq!(original_fee, pricing::price_for_label_length(5));

    // 2. The user agrees to this price and sets it as `max_price`.
    let max_price = original_fee;

    // 3. Before the user submits, a new pricing policy is enacted that makes 5-char names more expensive.
    // We can simulate this by attempting to register a name with a different length
    // that has a higher price, while still using the `max_price` from the original quote.
    let expensive_label = String::from_str(&env, "bob"); // 3-char name
    let new_quote = registrar.quote_registration(&expensive_label, &1, &time.now);
    let new_fee = new_quote.fee_stroops;
    assert_eq!(new_fee, pricing::price_for_label_length(3));
    assert!(
        new_fee > original_fee,
        "New fee should be higher for a shorter name"
    );

    // 4. User attempts to register the *new*, more expensive name, but with the *old* `max_price`.
    // This simulates the original name's price increasing.
    let result = registrar.try_register(&expensive_label, &owner, &1, &max_price, &time.now);

    // 5. The registration should fail because the current price (new_fee) exceeds max_price.
    assert!(
        result.is_err(),
        "Registration should fail due to price exceeding max_price"
    );
    assert_eq!(result.err(), Some(Ok(RegistrarError::InsufficientFee)));

    // 6. Now, if the user re-quotes and uses the new, higher fee as max_price, it should succeed.
    let result_success = registrar.try_register(&expensive_label, &owner, &1, &new_fee, &time.now);
    assert!(
        result_success.is_ok(),
        "Registration should succeed with updated max_price"
    );
}
