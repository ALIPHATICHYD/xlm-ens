# Threat Model: Registry Contract

**Status:** Draft  
**Contract:** `contracts/registry/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The Registry is the canonical source of truth for domain name ownership and lifecycle
state across the entire xlm-ens system. It tracks which address owns each `.xlm` name,
when registrations expire, and the grace-period window before a name becomes claimable
by a new registrant. All other contracts that need to make ownership decisions either
call the Registry directly or trust its output. A compromise of the Registry's state
integrity undermines every downstream contract.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Name ownership record | Maps name → owner, expiry, resolver | Critical | High |
| Owner name index | Maps owner → list of names owned | High | Medium |
| Expiry timestamps | `expires_at` and `grace_period_ends_at` | Critical | High |
| Admin key | Controls contract upgrades | Critical | High |
| Resolver address per name | Points to the Resolver contract for that name | High | Medium |
| Target address per name | The primary on-chain address for the name | High | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `require_auth()` on stored admin address |
| Registrar | `register`, `renew` | `owner.require_auth()` — owner must co-sign; Registrar orchestrates |
| Name owner (active) | `transfer`, `set_resolver`, `set_target_address`, `set_metadata`, `renew`, `burn` | `caller.require_auth()` checked inside `ensure_owner()` |
| Anyone | `burn` during Claimable state | No auth — open to all to free stale names |
| Public | All read functions (`resolve`, `name_state`, `check_owner`, `names_for_owner`) | None |

The `now_unix` timestamp is supplied by the caller. The Registry trusts it without
on-chain time validation. Downstream logic must treat it as caller-provided.

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| `xlm_ns_common` validation library | Compile-time | FQDN and timestamp validation bypassed |
| Soroban runtime | Critical | Storage, auth, and event integrity |
| Registrar contract | High — calls `register`/`renew` with owner auth | Could register names at wrong timestamps or fees |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-REG-01 | Attacker impersonates name owner to call `transfer` | Critical | Low | Mitigated | `ensure_owner()` calls `caller.require_auth()` + checks stored owner |
| S-REG-02 | Attacker impersonates admin to call `upgrade` | Critical | Low | Mitigated | `admin.require_auth()` enforced |
| S-REG-03 | Registrar supplies a fabricated `owner` address to `register` | High | Low | Mitigated | `owner.require_auth()` is called inside `register`; owner must co-sign |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-REG-01 | Caller sets `expires_at` to u64::MAX during registration | High | Low | Mitigated | `validate_lifecycle_timestamps()` requires `expires_at > now_unix` and `grace_period_ends_at > expires_at`; no upper-bound cap |
| T-REG-02 | Caller passes `now_unix` far in the past to manipulate lifecycle state | High | Medium | Open | `now_unix` is caller-supplied; no on-chain clock. Registrar should constrain the valid range |
| T-REG-03 | Non-owner mutates metadata URI, resolver, or target address | Critical | Low | Mitigated | All setters call `ensure_owner()` |
| T-REG-04 | Owner index grows inconsistent (stale entry after transfer/burn) | Medium | Low | Mitigated | `remove_from_owner_index()` called on transfer and burn; `audit_owner_index` provides detection |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-REG-01 | Name transfer disputed; no on-chain evidence | Medium | Low | Partial | `name:transfer` event emitted; no `name:register` event from Registry itself (Registrar emits registration events) |
| R-REG-02 | Admin upgrade disputed | Low | Low | Mitigated | `ContractUpgraded` event emitted with old/new version and admin address |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-REG-01 | Owner's portfolio of names is enumerable | Low | High | Accepted | `names_for_owner` is public by design; all on-chain storage is readable |
| I-REG-02 | Metadata URI exposes sensitive off-chain endpoint | Medium | Medium | Accepted | URI content is owner-supplied; contract has no way to restrict it |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-REG-01 | Attacker registers thousands of names under one address, making `names_for_owner` unbounded | Medium | Medium | Open | No pagination or cap on `names_for_owner`; large return value increases call cost |
| D-REG-02 | Attacker burns all names during grace period, disrupting legitimate renewals | Medium | Low | Accepted | Any caller can burn a Claimable name by design; grace period gives owner time to renew |
| D-REG-03 | Registrar sends inflated `expires_at` locking a name for decades | Low | Low | Accepted | Registration years are capped by Registrar's `validate_registration_years_soroban`; Registry itself applies no upper cap |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-REG-01 | Attacker calls `register` directly (bypassing Registrar payment) | High | Medium | Partial | `register` requires `owner.require_auth()` so the attacker must be the owner; however a free registration bypassing fee collection is possible if the caller is the owner. Registrar is the intended gatekeeper |
| E-REG-02 | Attacker uses stale owner index to assert ownership after transfer | Medium | Low | Mitigated | `ensure_owner()` reads the live `Entry`, not the index; index is only used for enumeration |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call by presence of `Admin` key | First-call race on deployment |
| `upgrade` | Admin | `admin.require_auth()` | Admin key compromise |
| `register` | Owner (via Registrar) | `owner.require_auth()` | Free registration if called directly |
| `resolve` | Public | None | Read-only |
| `name_state` | Public | None | Read-only |
| `transfer` | Owner | `caller.require_auth()` | Ownership spoofing |
| `set_resolver` | Owner | `caller.require_auth()` | Redirect resolution |
| `set_target_address` | Owner | `caller.require_auth()` | Misdirect payments |
| `set_metadata` | Owner | `caller.require_auth()` | Metadata injection |
| `renew` | Owner (active or grace) | `caller.require_auth()` | Renewal bypass |
| `burn` | Owner (active) or Anyone (claimable) | Owner: `caller.require_auth()`; Claimable: none | Griefing during grace period |
| `names_for_owner` | Public | None | Unbounded response |
| `audit_owner_index` | Public | None | Read-only consistency check |

---

## 7. Open Issues

1. **T-REG-02 — Caller-supplied `now_unix`**: The Registry accepts `now_unix` without
   on-chain clock verification. A caller who controls a Registrar-like contract could
   supply a manipulated timestamp to register a name that is not yet claimable or extend
   an expiry from a point in the past. Recommended fix: compare `now_unix` against
   `env.ledger().timestamp()` and reject values that deviate by more than a tolerance
   window (e.g., ±5 minutes).

2. **E-REG-01 — Direct `register` bypasses fee collection**: Any address that holds the
   owner key can call `register` directly without going through the Registrar, paying no
   fee. The Registry should verify the caller is the authorized Registrar contract, or the
   Registrar must be the sole permitted address for `register`.

3. **D-REG-01 — Unbounded `names_for_owner`**: No pagination cap. Recommended fix: accept
   an `offset` and `limit` parameter, or cap the return at a constant (e.g., 200 names).

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
