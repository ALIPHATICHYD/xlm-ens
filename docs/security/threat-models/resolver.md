# Threat Model: Resolver Contract

**Status:** Draft  
**Contract:** `contracts/resolver/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The Resolver maps `.xlm` names to on-chain addresses, reverse mappings (address → name),
primary names, and arbitrary text records. It cross-checks ownership against the Registry
on every mutation so that only the current name owner can update resolution data. A
compromise of the Resolver allows attackers to redirect payments, impersonate identities,
and corrupt the namespace.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Forward records | name → addresses + text records | Critical | High |
| Reverse mappings | address → name | High | High |
| Primary name pointers | address → canonical primary name | High | High |
| Allowed key schema | Admin-managed set of valid text-record keys | High | Medium |
| Admin key | Controls upgrades | Critical | High |
| Registry address | Cross-contract dependency for ownership verification | Critical | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `admin.require_auth()` |
| Name owner | All mutating functions | `assert_owner()` equality check — but **missing `caller.require_auth()`** (see Open Issues) |
| Public | `resolve`, `reverse`, `has_record`, `batch_resolve`, `batch_reverse`, `get_address` | None |

The `assert_owner` helper checks that `caller` matches either the registry-backed owner or the stored record owner, but **never calls `caller.require_auth()`**. Any transaction can supply an arbitrary `caller` address and pass the equality check if it knows the legitimate owner's address.

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| Registry contract | High — cross-contract call to verify ownership | Attacker who compromises registry can spoof ownership to mutate any record |
| Soroban runtime | Critical | Storage, event, and auth primitives |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-RES-01 | Attacker supplies `caller=<victim>` to `set_address`, `set_text_record`, `remove_record`, `update_owner`, `set_primary_name` without victim's signature | Critical | High | **Open** | `assert_owner` checks equality but never calls `caller.require_auth()`; any caller can impersonate any owner |
| S-RES-02 | Attacker supplies `caller=<victim>` to `transfer_record_owner` to steal a record | Critical | High | **Open** | `transfer_record_owner` checks `record.owner != caller` with no `require_auth()`; same root cause as S-RES-01 |
| S-RES-03 | Attacker impersonates admin to upgrade | Critical | Low | Mitigated | `admin.require_auth()` correctly enforced in `upgrade` |
| S-RES-04 | Compromised registry reports wrong owner, allowing attacker to mutate victim's record | Critical | Low | Mitigated by registry integrity | Depends on Registry not being compromised (see registry threat model) |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-RES-01 | Attacker writes arbitrary Stellar address into victim's forward record, redirecting payments | Critical | High | **Open** | Follows from S-RES-01; no `require_auth()` on `set_record`/`set_address` |
| T-RES-02 | Attacker poisons victim's text records (e.g. email, url) to phish contacts | High | High | **Open** | Follows from S-RES-01; no `require_auth()` on `set_text_record` |
| T-RES-03 | Attacker removes victim's forward record, causing lookups to return `None` | High | High | **Open** | Follows from S-RES-01; no `require_auth()` on `remove_record` |
| T-RES-04 | Attacker sets `now_unix=0` in `set_primary_name`/`remove_record`/`update_owner` to bypass time-gated ownership checks | Medium | Low | Accepted | These functions pass `now_unix=0` which is hardcoded; the underlying risk is subsumed by S-RES-01 |
| T-RES-05 | Caller-supplied `now_unix` in `set_record`/`set_address`/`batch_set` used to record a manipulated `updated_at` timestamp | Low | Medium | Accepted | `updated_at` is informational only; it does not affect ownership decisions |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-RES-01 | Record mutation by impersonating caller leaves no attribution | High | High | Partial | Events are emitted for all mutations; however the emitted `caller` field is the attacker-supplied address, not a verified signer |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-RES-01 | Text records (email, social handles) publicly readable | Low | High | Accepted | By design; users choose what to publish |
| I-RES-02 | Reverse lookup exposes the primary name for any address | Low | High | Accepted | By design; the mapping is user-controlled |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-RES-01 | Attacker removes all records for a victim's name | High | High | **Open** | Follows from S-RES-01; attacker can call `remove_record` impersonating any owner |
| D-RES-02 | `batch_set` exceeds `MAX_BATCH_OPS=16` limit returning `BatchTooLarge` | Low | Low | Mitigated | Batch size enforced; large batches are rejected before auth |
| D-RES-03 | Persistent entry TTL expires causing stale forward/reverse records to vanish | Medium | Low | Mitigated | `extend_persistent_ttl` called on every write; threshold is ~6 months before renewal |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-RES-01 | Attacker transfers record ownership to themselves via `transfer_record_owner` without victim signature | Critical | High | **Open** | No `require_auth()` on `transfer_record_owner`; attacker gains persistent ownership of victim's resolver record |
| E-RES-02 | Attacker hijacks primary name pointer to redirect all reverse lookups for victim address | High | High | **Open** | Follows from S-RES-01 applied to `set_primary_name` |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | Deployment race |
| `upgrade` | Admin | `admin.require_auth()` | Correctly gated |
| `set_record` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `set_address` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `set_text_record` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `set_primary_name` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `remove_record` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `update_owner` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `transfer_record_owner` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-02** |
| `batch_set` | Name owner | Equality check only — **no `require_auth()`** | **CRITICAL — S-RES-01** |
| `resolve` | Public | None | Read-only |
| `reverse` | Public | None | Read-only |
| `batch_resolve` | Public | None | Read-only |
| `batch_reverse` | Public | None | Read-only |
| `has_record` | Public | None | Read-only |
| `get_address` | Public | None | Read-only |
| `get_stellar_address` | Public | None | Read-only |

---

## 7. Open Issues

1. **S-RES-01 / T-RES-01–T-RES-03 / D-RES-01 / E-RES-02 — `caller.require_auth()` missing
   in `assert_owner`**: The `assert_owner` helper verifies that `caller` matches the
   owner address but never calls `caller.require_auth()`. This means any transaction can
   supply any address as `caller` without that address signing the transaction. Every
   mutating function that routes through `assert_owner` is affected: `set_address`,
   `set_text_record`, `set_primary_name`, `remove_record`, `update_owner`, `batch_set`.
   Fix: add `caller.require_auth()` at the top of `assert_owner`.

2. **S-RES-02 / E-RES-01 — `transfer_record_owner` missing `caller.require_auth()`**:
   `transfer_record_owner` independently lacks `caller.require_auth()`. An attacker can
   permanently steal any record's ownership by supplying the victim's address as `caller`.
   Fix: add `caller.require_auth()` before the `record.owner != caller` check.

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
