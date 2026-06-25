# Migration Guide: vX.Y → vX.Z

**Affected contracts:** _list each contract that changed_  
**Affected SDK version:** `xlm-ns-sdk vX.Y → vX.Z`  
**Affected CLI version:** `xlm-ns-cli vX.Y → vX.Z`  
**Severity:** Breaking / Non-breaking / Additive  
**Date:** YYYY-MM-DD

---

## Overview

_One paragraph: what changed and why._

---

## Breaking Changes

List every change that will cause existing callers to fail without code updates.

### `ContractName` — Changed: `function_name`

**Before (vX.Y):**

```rust
// old signature
fn function_name(env: Env, param_a: TypeA) -> Result<ReturnType, Error>
```

**After (vX.Z):**

```rust
// new signature — param_b added; TypeA replaced by TypeB
fn function_name(env: Env, param_b: TypeB, param_a: TypeA) -> Result<NewReturn, Error>
```

**Why it changed:** _Explain the reason._

**Migration:** Update all call sites to pass the new `param_b` argument. If you
use the SDK, upgrade to `xlm-ns-sdk vX.Z` and regenerate any bindings.

---

### `ContractName` — Removed: `old_function`

`old_function` was removed. Use `new_function` instead.

**Before:**

```rust
contract.old_function(env, arg);
```

**After:**

```rust
contract.new_function(env, new_arg_mapping(arg));
```

---

### `ContractName` — Changed: error code renumbering

| Old name | Old code | New name | New code |
|----------|----------|----------|----------|
| `ErrorA` | `3` | `ErrorA` | `5` |

If your code matches on numeric error codes rather than enum variants, update
the literals.

---

## Deprecations

Changes that still work in vX.Z but will be removed in the next minor version.

### `ContractName` — Deprecated: `old_query`

`old_query` is deprecated in favour of `new_query`. It will be removed in vX.(Z+1).

```rust
// deprecated — still works but logs a warning event
contract.old_query(env, arg);

// preferred replacement
contract.new_query(env, arg, extra_param);
```

---

## New Additions

Non-breaking additions that are only available from vX.Z onward.

### `ContractName` — Added: `new_function`

```rust
fn new_function(env: Env, name: String) -> Result<NewType, Error>
```

_Describe what it does and when to use it._

---

## Storage Migration

If `CONTRACT_VERSION` was incremented, on-chain state requires a migration step
before the new contract can read old records.

### Steps

1. Call `upgrade(env, new_wasm_hash, migration_data)` with the `migration_data`
   encoding the target version as a big-endian `u32` in the first four bytes.
2. Verify the new on-chain `CONTRACT_VERSION` with `get_version()`.
3. Run any off-chain reindexing jobs against the new storage layout.

### Encoding `migration_data`

```rust
let target_version: u32 = 2;
let migration_data = Bytes::from_array(&env, &target_version.to_be_bytes()
    .into_iter().chain([0u8; 28]).collect::<Vec<_>>().try_into().unwrap());
```

---

## SDK Upgrade Steps

1. Update `Cargo.toml`:

   ```toml
   xlm-ns-sdk = "X.Z"
   ```

2. Run `cargo build` and address any compilation errors from the breaking changes
   listed above.

3. Re-run your integration tests against testnet with the new contract addresses.

---

## CLI Upgrade Steps

1. Download the `vX.Z` CLI binary from the [Releases](https://github.com/Soroban-Ens/xlm-ens/releases) page.
2. Verify the checksum against `SHA256SUMS.txt`.
3. Replace the existing binary.
4. Confirm the version: `xlm-ns-cli --version`.

---

## Rollback

If issues are found after upgrading:

1. Contracts: on-chain state cannot be rolled back without redeploying the old
   WASM. Coordinate with testnet operators before upgrading in production.
2. SDK/CLI: pin to the previous version in `Cargo.toml` or re-download the old
   binary.

---

## Changelog Reference

See [CHANGELOG.md](../../CHANGELOG.md) for the complete list of changes in this
release.
