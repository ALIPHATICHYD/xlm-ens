# Threat Model: Bridge Contract

**Status:** Draft  
**Contract:** `contracts/bridge/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The Bridge contract is a message-builder for cross-chain name resolution via the Axelar
General Message Passing (GMP) protocol. It stores per-chain routing records
(`destination_chain`, `destination_resolver`, `gateway`) and constructs forward and
reverse GMP message payloads. It does not hold funds or execute cross-chain calls directly
— it only builds the string payload that a caller is expected to submit to Axelar. The
primary risk is producing malformed or attacker-controlled routing that misdirects
cross-chain resolution queries.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Chain routing table | `chain → BridgeRoute` (destination_chain, destination_resolver, gateway) | High | Medium |
| Admin key | Controls upgrades | Critical | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `admin.require_auth()` |
| Anyone | `register_chain` | **None** — but routes are hardcoded in `target_for_chain` (see note below) |
| Public | `build_message`, `build_reverse_message`, `route` | None |

**Note on `register_chain`**: Although there is no auth check, the stored route is
always taken from the internal `target_for_chain` helper which returns a hardcoded
`BridgeRoute` based on the chain name. An unauthenticated caller cannot inject custom
routing data — they can only trigger storage of hardcoded routes. The lack of auth is
therefore lower severity than it appears, but it remains an unnecessary open surface.

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| `xlm_ns_common` chain/FQDN validation | Compile-time | Invalid chain names or names accepted |
| Axelar GMP protocol | Critical — message is submitted externally by caller | Malformed message payloads could be silently dropped or misrouted |
| Destination resolver contracts (off-chain) | High — hardcoded addresses | Hardcoded `0xbaseResolver`, `0xethResolver`, `0xarbResolver` are placeholder values; production deployment must replace these |
| Soroban runtime | Critical | Storage and execution |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-BRG-01 | Attacker calls `build_message` for a name they don't own to probe routing | Low | High | Accepted | `build_message` only constructs a payload string; it does not verify the caller owns the name or submit anything to Axelar |
| S-BRG-02 | Attacker impersonates admin to upgrade | Critical | Low | Mitigated | `admin.require_auth()` enforced in `upgrade` |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-BRG-01 | Hardcoded resolver addresses are placeholders; production deployment with wrong addresses silently misdirects all cross-chain resolutions | Critical | High | **Open** | `0xbaseResolver`, `0xethResolver`, `0xarbResolver` are placeholder strings; production deployment must update `target_for_chain` with real addresses |
| T-BRG-02 | `register_chain` has no auth; any address can trigger re-storage of hardcoded routes | Low | High | Partial | Routes are hardcoded in `target_for_chain`, not caller-supplied; re-storage is idempotent and benign today, but the pattern is dangerous if `target_for_chain` is ever replaced with caller-supplied data |
| T-BRG-03 | `build_message` / `build_reverse_message` produce outputs not validated before submission to Axelar | Medium | Low | Accepted | The contract validates FQDN and chain name; the payload format is deterministic; off-chain submission is the caller's responsibility |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-BRG-01 | `register_chain` emits no event; chain route activation is untracked | Medium | High | Open | No event emitted from `register_chain`; add an event for auditability |
| R-BRG-02 | `build_message` emits no event; message construction is untracked | Low | High | Accepted | Message building is read-only; caller is responsible for submitting and recording the message |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-BRG-01 | Stored resolver and gateway addresses are publicly readable | Low | High | Accepted | All storage is public on-chain; these are infrastructure addresses, not secrets |
| I-BRG-02 | GMP message payload includes the plain-text name and destination resolver; readable in mempool | Low | High | Accepted | By design; GMP messages are not confidential |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-BRG-01 | `register_chain` called repeatedly for all three supported chains with no real effect; minor gas waste | Low | Low | Accepted | Idempotent; no meaningful state change |
| D-BRG-02 | Unsupported chain name passes validation but `register_chain` returns `UnsupportedChain`; silently limits chain coverage | Medium | Low | Accepted | `target_for_chain` only supports `base`, `ethereum`, `arbitrum`; new chains require contract upgrade |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-BRG-01 | Attacker with admin key upgrades contract to inject malicious destination resolver, redirecting all future cross-chain resolutions | Critical | Low | Accepted (admin trust) | Admin key compromise is the root; multi-sig admin mitigates this |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | Deployment race |
| `upgrade` | Admin | `admin.require_auth()` | Correctly gated |
| `register_chain` | Anyone | None | Routes are hardcoded; risk is low today |
| `build_message` | Public | None | Read-only payload builder |
| `build_reverse_message` | Public | None | Read-only payload builder |
| `route` | Public | None | Read-only |

---

## 7. Open Issues

1. **T-BRG-01 — Placeholder resolver addresses**: `target_for_chain` hardcodes
   `0xbaseResolver`, `0xethResolver`, `0xarbResolver` as destination resolver addresses.
   These are not real contract addresses. The contract must be upgraded with correct
   production addresses before mainnet deployment; any cross-chain message built with
   these values will be routed to non-existent contracts.

2. **T-BRG-02 — `register_chain` unauthenticated**: Although today's routes are
   hardcoded, the open surface creates a pattern that is dangerous if `target_for_chain`
   is later refactored to accept caller-supplied data. Add `admin.require_auth()` as a
   prophylactic.

3. **R-BRG-01 — No event from `register_chain`**: Add a `ChainRegistered` event so
   route activations are auditable on-chain.

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
