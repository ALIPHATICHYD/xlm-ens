# Threat Model: Registrar Contract

**Status:** Draft  
**Contract:** `contracts/registrar/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The Registrar is the business-logic layer for name registration and renewal. It enforces
pricing policy (tiered by label length), manages reserved labels, tracks treasury
accumulation, and applies rate limiting to prevent bulk registration abuse. Registrations
are finalized by calling through to the Registry contract. The Registrar is the only
intended caller of `Registry.register` and `Registry.renew`.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Treasury balance | Accumulated registration and renewal fees (u64 stroops) | Critical | High |
| Reserved label set | Labels blocked from public registration | High | Medium |
| Registration records | Per-name metadata: owner, dates, fee paid | High | High |
| Rate limit config | Window size and max registrations per window | High | Medium |
| Whitelist | Addresses that bypass rate limiting | High | Medium |
| Admin key | Controls upgrades | Critical | High |
| Registry address | Cross-contract dependency for finalizing registrations | Critical | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `admin.require_auth()` |
| Name owner | `register`, `renew` | `owner.require_auth()` / `caller.require_auth()` |
| Anyone | `set_rate_limit_config`, `whitelist_address`, `remove_whitelist_address` | **None — CRITICAL BUG (see Open Issues)** |
| Anyone | `reserve_label`, `load_reserved_manifest` | **None — open write (see Open Issues)** |
| Public | All read functions | None |

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| Registry contract | Critical — finalizes ownership | Registrar calls could be replayed or re-ordered |
| Soroban runtime | Critical | Storage and auth primitives |
| `xlm_ns_common` pricing/validation | Compile-time | Incorrect fee calculation or label acceptance |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-RAR-01 | Attacker impersonates admin to upgrade contract | Critical | Low | Mitigated | `admin.require_auth()` enforced |
| S-RAR-02 | Attacker impersonates owner to renew someone else's name | High | Low | Mitigated | `caller.require_auth()` and `record.owner == caller` check in `renew` |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-RAR-01 | Attacker calls `set_rate_limit_config` with window=0 or max=0, creating a permanent DoS | Critical | High | **Open** | No `require_auth()` on governance functions; any caller can set these values |
| T-RAR-02 | Attacker calls `whitelist_address` to add their own address, bypassing rate limits | Critical | High | **Open** | No `require_auth()` on `whitelist_address` |
| T-RAR-03 | Attacker calls `remove_whitelist_address` to remove legitimate addresses | High | High | **Open** | No `require_auth()` on `remove_whitelist_address` |
| T-RAR-04 | Attacker calls `reserve_label` or `load_reserved_manifest` to block legitimate labels | High | High | **Open** | No auth on `reserve_label` or `load_reserved_manifest`; anyone can reserve any label |
| T-RAR-05 | Attacker passes manipulated `now_unix` to `register` to bypass rate-limit window | High | Medium | Open | Rate-limit window uses caller-supplied `now_unix`; no on-chain clock cross-check |
| T-RAR-06 | Registration fee overpayment silently retained; no refund | Low | Medium | Accepted | Excess payment added to treasury by design; callers should quote first |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-RAR-01 | Rate limit config change untraceable | Medium | High | Partial | `registrar:rate` event emitted, but no auth, so event could be from anyone |
| R-RAR-02 | Whitelist modification untraceable to admin | Medium | High | Partial | `registrar:wlist`/`registrar:unwlist` events emitted but caller not validated |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-RAR-01 | Treasury balance visible to all | Low | High | Accepted | All on-chain storage is public; no sensitive value here |
| I-RAR-02 | Registration records reveal fee paid and registration timing | Low | High | Accepted | By design for transparency |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-RAR-01 | Rate limit set to 0 by attacker, blocking all new registrations | Critical | High | **Open** | Consequence of T-RAR-01; no auth on `set_rate_limit_config` |
| D-RAR-02 | Attacker reserves all short labels (1–4 char) blocking premium names | High | High | **Open** | No auth on `reserve_label`; attacker pays nothing to poison the reserved set |
| D-RAR-03 | Treasury balance permanently locked — no withdrawal function exists | High | Low | **Open** | Fees accumulate but there is no `withdraw_treasury` function; funds are irrecoverable without upgrade |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-RAR-01 | Attacker whitelists own address to bypass rate limits and bulk-register | Critical | High | **Open** | Follows from T-RAR-02; no auth on whitelist |
| E-RAR-02 | Attacker reserves a label the real owner wants, forcing them to contact admin | High | High | **Open** | Follows from T-RAR-04; no auth on reserve |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | Deployment race |
| `upgrade` | Admin | `admin.require_auth()` | Admin key compromise |
| `register` | Owner | `owner.require_auth()` | Fee bypass if called without Registrar |
| `renew` | Name owner | `caller.require_auth()` | Renewal pricing |
| `reserve_label` | **Anyone** | **None** | **CRITICAL — see T-RAR-04** |
| `load_reserved_manifest` | **Anyone** | **None** | **CRITICAL — see T-RAR-04** |
| `set_rate_limit_config` | **Anyone** | **None** | **CRITICAL — see T-RAR-01** |
| `whitelist_address` | **Anyone** | **None** | **CRITICAL — see T-RAR-02** |
| `remove_whitelist_address` | **Anyone** | **None** | **CRITICAL — see T-RAR-03** |
| `quote_registration` | Public | None | Read-only |
| `treasury_balance` | Public | None | Read-only |

---

## 7. Open Issues

1. **T-RAR-01 / D-RAR-01 — `set_rate_limit_config` unauthenticated**: Any caller can
   disable registration by setting `max_registrations_per_window = 0` or setting
   `window_size_seconds = u64::MAX`. Add `admin.require_auth()` to this function.

2. **T-RAR-02 / E-RAR-01 — `whitelist_address` unauthenticated**: Any caller can add
   their address to the rate-limit bypass list. Add `admin.require_auth()`.

3. **T-RAR-03 — `remove_whitelist_address` unauthenticated**: Any caller can remove
   legitimate addresses from the whitelist. Add `admin.require_auth()`.

4. **T-RAR-04 / D-RAR-02 / E-RAR-02 — `reserve_label`/`load_reserved_manifest`
   unauthenticated**: Any caller can permanently reserve any label, blocking legitimate
   registrations. Add `admin.require_auth()` to both.

5. **D-RAR-03 — No treasury withdrawal function**: Fees accumulate with no way to extract
   them without a contract upgrade. Add a `withdraw_treasury(env, recipient, amount)`
   function gated on `admin.require_auth()`.

6. **T-RAR-05 — Caller-supplied `now_unix`**: Rate-limit windows use caller-provided time.
   An attacker who submits `now_unix` just past a window boundary gets a fresh window.
   Validate against `env.ledger().timestamp()` with a tolerance.

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
