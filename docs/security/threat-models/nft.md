# Threat Model: NFT Contract

**Status:** Draft  
**Contract:** `contracts/nft/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The NFT contract represents `.xlm` name registrations as non-fungible tokens. Each
registered name maps to a `TokenRecord` with an owner, an optional approved-spender
address, and an optional metadata URI. The contract implements ERC-721-style ownership
(owner transfer, operator approval, `transfer_from`). Because token ownership is the
root of identity in the xlm-ens system, auth failures here are equivalent to arbitrary
name theft.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Token ownership | `token_id → owner` | Critical | High |
| Approved-spender records | One approved address per token | High | Medium |
| Metadata URIs | Off-chain pointer per token | Medium | Low |
| Token ID index | Global list for enumeration | Medium | Medium |
| Owner token index | Per-address token list | Medium | Medium |
| Admin key | Controls upgrades | Critical | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `admin.require_auth()` |
| Anyone | `mint` (any token_id, any owner) | **None — CRITICAL BUG (see Open Issues)** |
| Token owner | `approve`, `approve_clear`, `transfer` | Equality check only — **missing `caller.require_auth()`** |
| Approved spender | `transfer_from` | Equality check only — **missing `spender.require_auth()`** |
| Public | `owner_of`, `token`, `balance_of`, `total_supply`, `token_by_index`, `token_of_owner_by_index`, `token_uri` | None |

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| Soroban runtime | Critical | Storage, auth, and event primitives |
| Name registry (not integrated) | None — NFT does not cross-call registry | Token and Registry ownership can diverge |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-NFT-01 | Attacker mints any token_id with any owner address without any authorization | Critical | High | **Open** | `mint` has no `require_auth()` at all; any caller can mint any token |
| S-NFT-02 | Attacker supplies `caller=<owner>` to `approve`/`approve_clear` without owner's signature | Critical | High | **Open** | Equality check `record.owner != caller` with no `caller.require_auth()` |
| S-NFT-03 | Attacker supplies `caller=<owner>` to `transfer` to steal a token | Critical | High | **Open** | Equality checks with no `caller.require_auth()` or `spender.require_auth()` |
| S-NFT-04 | Attacker uses `transfer_from` impersonating an approved spender | Critical | High | **Open** | Checks `record.approved == Some(&spender)` but no `spender.require_auth()` |
| S-NFT-05 | Attacker impersonates admin to upgrade | Critical | Low | Mitigated | `admin.require_auth()` enforced |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-NFT-01 | Attacker mints a token for a registered name, creating a duplicate NFT not tied to the real owner | Critical | High | **Open** | No `require_auth()` on `mint`; no cross-check with Registry; anyone can mint any ID |
| T-NFT-02 | Attacker grants themselves approval on any token by impersonating the owner | Critical | High | **Open** | Follows from S-NFT-02; approved spender can then call `transfer_from` |
| T-NFT-03 | NFT and Registry ownership can permanently diverge | High | High | **Open** | NFT minting and Registry registration are independent; no sync mechanism |
| T-NFT-04 | Metadata URI overwrite — no setter function exists so URI set at mint is permanent | Low | Low | Accepted | `metadata_uri` is immutable after mint; no updater function |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-NFT-01 | `mint` event emits `owner` and `caller` but neither is authenticated | High | High | **Open** | No auth on `mint`; event data is unverifiable |
| R-NFT-02 | Transfer disputed; events emitted from `events::transfer` but `caller` is unauthenticated | High | High | **Open** | Follows from S-NFT-03 |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-NFT-01 | All token ownership and approvals are publicly readable | Low | High | Accepted | Standard NFT behavior; all on-chain storage is public |
| I-NFT-02 | Metadata URIs pointing to off-chain services may expose PII | Medium | Low | Accepted | URI content is owner-supplied and off-chain; contract cannot restrict it |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-NFT-01 | Attacker mints thousands of dummy tokens filling `TokenIds` and `OwnerTokens` indices | High | High | **Open** | No auth on `mint`; `total_supply` and `balance_of` grow without bound |
| D-NFT-02 | `total_supply` and `token_by_index` become unusable with large token sets | Medium | High | **Open** | Index is unbounded and loaded fully; large token counts exhaust instruction budget |
| D-NFT-03 | Attacker mints a token_id matching a name already registered in Registry | High | High | **Open** | Creates phantom NFT colliding with the real one; clients can't distinguish legitimate vs. spurious |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-NFT-01 | Attacker mints a token for any name and then transfers it, falsely claiming NFT-based ownership | Critical | High | **Open** | Follows from S-NFT-01; downstream systems that trust NFT ownership are compromised |
| E-NFT-02 | Attacker approves themselves on a victim's token via impersonation then drains via `transfer_from` | Critical | High | **Open** | Follows from S-NFT-02 + S-NFT-04 |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | Deployment race |
| `upgrade` | Admin | `admin.require_auth()` | Correctly gated |
| `mint` | **Anyone** | **None — no auth** | **CRITICAL — S-NFT-01, T-NFT-01, E-NFT-01** |
| `approve` | Token owner | Equality check — **no `require_auth()`** | **CRITICAL — S-NFT-02** |
| `approve_clear` | Token owner | Equality check — **no `require_auth()`** | **CRITICAL — S-NFT-02** |
| `transfer` | Token owner or approved | Equality check — **no `require_auth()`** | **CRITICAL — S-NFT-03** |
| `transfer_from` | Approved spender | Equality check — **no `require_auth()`** | **CRITICAL — S-NFT-04** |
| `owner_of` | Public | None | Read-only |
| `token` | Public | None | Read-only |
| `balance_of` | Public | None | Read-only |
| `total_supply` | Public | None | Read-only |
| `token_by_index` | Public | None | Read-only |
| `token_of_owner_by_index` | Public | None | Read-only |
| `token_uri` | Public | None | Read-only |

---

## 7. Open Issues

1. **S-NFT-01 / T-NFT-01 / D-NFT-01 / E-NFT-01 — `mint` has no authorization**: Any
   caller can mint any token ID for any owner without authentication. Add either
   `admin.require_auth()` (admin-only minting) or require the Registrar to sign via an
   authorized-minter mechanism. Also add a cross-check that the `token_id` is a validly
   registered name in the Registry.

2. **S-NFT-02 / T-NFT-02 / E-NFT-02 — `approve`/`approve_clear` missing
   `caller.require_auth()`**: Equality check is correct but caller signature is never
   verified. Add `caller.require_auth()` to both functions.

3. **S-NFT-03 — `transfer` missing `caller.require_auth()`**: Add `caller.require_auth()`
   before the ownership/approval check.

4. **S-NFT-04 — `transfer_from` missing `spender.require_auth()`**: Add
   `spender.require_auth()` before the spender/owner check.

5. **T-NFT-03 — NFT and Registry ownership diverge**: There is no synchronization between
   NFT ownership and Registry ownership. A name transfer in the Registry does not update
   the NFT. Consider making the Registry the canonical source and having the NFT contract
   verify ownership against it, or integrating NFT transfer into the Registry's `transfer`
   function.

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
