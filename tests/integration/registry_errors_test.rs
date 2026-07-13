//! Negative tests for the Registry contract, ensuring all error paths are covered.

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use xlm_ns_registry::{RegistryContract, RegistryContractClient, RegistryError};

const DEFAULT_TTL: u64 = 365 * 24 * 3600; // 1 year
const DEFAULT_GRACE_PERIOD: u64 = 30 * 24 * 3600; // 30 days

fn setup(env: &Env) -> RegistryContractClient<'static> {
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    client
}

fn register_domain(env: &Env, registry: &RegistryContractClient, name_str: &str) -> (Address, u64) {
    let owner = Address::generate(env);
    let now = env.ledger().timestamp();
    let name = String::from_str(env, name_str);
    registry.register(
        &name,
        &owner,
        &None::<String>,
        &None::<String>,
        &now,
        &(now + DEFAULT_TTL),
        &(now + DEFAULT_TTL + DEFAULT_GRACE_PERIOD),
    );
    (owner, now)
}

#[test]
fn test_register_fails_if_already_registered() {
    let env = Env::default();
    env.mock_all_auths();

    let registry = setup(&env);
    let (owner, now) = register_domain(&env, &registry, "test.xlm");
    let name = String::from_str(&env, "test.xlm");

    // Attempt to register the same name again
    let res = registry.try_register(
        &name,
        &owner,
        &None::<String>,
        &None::<String>,
        &now,
        &(now + DEFAULT_TTL),
        &(now + DEFAULT_TTL + DEFAULT_GRACE_PERIOD),
    );

    assert!(matches!(res, Err(Ok(RegistryError::AlreadyRegistered))));
}

#[test]
fn test_transfer_fails_if_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let registry = setup(&env);
    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let name = String::from_str(&env, "nonexistent.xlm");

    let res = registry.try_transfer(&name, &owner, &new_owner, &env.ledger().timestamp());

    assert!(matches!(res, Err(Ok(RegistryError::NotFound))));
}

#[test]
fn test_burn_fails_if_not_yet_claimable() {
    let env = Env::default();
    env.mock_all_auths();

    let registry = setup(&env);
    let (_owner, _) = register_domain(&env, &registry, "test.xlm");
    let name = String::from_str(&env, "test.xlm");

    // A non-owner caller cannot burn an active (not yet claimable) name.
    let stranger = Address::generate(&env);
    let res = registry.try_burn(&name, &stranger, &env.ledger().timestamp());

    assert!(matches!(res, Err(Ok(RegistryError::Unauthorized))));
}

#[test]
fn test_transfer_fails_if_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let registry = setup(&env);
    let (owner, now) = register_domain(&env, &registry, "test.xlm");
    let new_owner = Address::generate(&env);
    let name = String::from_str(&env, "test.xlm");

    // Time past expiry into grace period
    let future_time = now + DEFAULT_TTL + 1;

    let res = registry.try_transfer(&name, &owner, &new_owner, &future_time);

    assert!(matches!(res, Err(Ok(RegistryError::NotActive))));
}

#[test]
fn test_transfer_fails_if_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let registry = setup(&env);
    let (_owner, _) = register_domain(&env, &registry, "test.xlm");
    let attacker = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let name = String::from_str(&env, "test.xlm");

    // Attacker (not owner) tries to transfer
    let res = registry.try_transfer(
        &name,
        &attacker, // Unauthorized caller
        &new_owner,
        &env.ledger().timestamp(),
    );

    assert!(matches!(res, Err(Ok(RegistryError::Unauthorized))));
}

#[test]
fn test_register_fails_if_metadata_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let registry = setup(&env);
    let owner = Address::generate(&env);
    let name = String::from_str(&env, "test2.xlm");
    let now = env.ledger().timestamp();

    // From common/src/lib.rs, MAX_METADATA_LEN is 1024
    let long_metadata = "a".repeat(1025);
    let long_metadata_str = String::from_str(&env, &long_metadata);

    let res = registry.try_register(
        &name,
        &owner,
        &None::<String>,
        &Some(long_metadata_str),
        &now,
        &(now + DEFAULT_TTL),
        &(now + DEFAULT_TTL + DEFAULT_GRACE_PERIOD),
    );

    assert!(matches!(res, Err(Ok(RegistryError::MetadataTooLong))));
}
