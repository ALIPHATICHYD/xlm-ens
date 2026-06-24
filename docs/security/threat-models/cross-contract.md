# Cross-Contract Interaction Risks

**Status:** Draft  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Overview

This document covers threats that arise specifically from interactions between the seven
xlm-ens smart contracts. Individual contracts are analyzed in their own threat model
documents; this document focuses on the boundaries between them: trust assumptions when
one contract calls another, state divergence between contracts, and emergent threats that
only exist because multiple contracts compose.

---

## 2. Contract Interaction Map

```
                    ┌─────────────┐
                    │  Registrar  │──────────────────────────────────┐
                    └──────┬──────┘                                  │
                           │ register / renew                        │
                           ▼                                         ▼
                    ┌─────────────┐           ┌────────────────────────────┐
                    │  Registry   │◄──────────│  Resolver (assert_owner)   │
                    └──────┬──────┘  resolve  └────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
    ┌──────────┐    ┌──────────┐    ┌──────────────┐
    │   NFT    │    │Subdomain │    │   Auction    │
    └──────────┘    └──────────┘    └──────────────┘
                                          │
                                          ▼ (message only)
                                    ┌──────────┐
                                    │  Bridge  │
                                    └──────────┘
```

Key observations:
- **Resolver → Registry**: cross-contract call to `resolve` for ownership verification.
- **Registrar → Registry**: calls `register`/`renew` to finalize ownership records.
- **NFT, Subdomain, Auction, Bridge**: standalone; they do NOT call the Registry to
  verify ownership. Any name-based authorization in these contracts is self-contained
  and can diverge from Registry truth.

---

## 3. Cross-Contract Threat Analysis

### 3.1 Registry → Resolver trust channel

| ID | Threat | Severity | Likelihood | Status | Notes |
|----|--------|----------|------------|--------|-------|
| X-01 | Compromised Registry reports wrong owner; Resolver grants mutations to attacker | Critical | Low | Conditional | Resolver fully trusts Registry's `resolve` response; no independent verification |
| X-02 | Registry initialized with wrong `registry` address in Resolver; ownership checks silently fail | High | Low | Mitigated | `initialize` can only be called once; deployer is responsible for correct address |
| X-03 | Resolver cross-call to Registry panics (e.g. Registry not deployed or upgraded); all Resolver mutations fail | High | Low | Mitigated | `registry_owner` returns `None` on `NotInitialized`, falling back to stored record owner |

### 3.2 Registrar → Registry trust channel

| ID | Threat | Severity | Likelihood | Status | Notes |
|----|--------|----------|------------|--------|-------|
| X-04 | Direct `Registry.register` call bypasses Registrar fee collection | High | Medium | Open | Registry only checks `owner.require_auth()`; no check that caller is the authorized Registrar. Any address that controls the owner key can self-register for free. |
| X-05 | Registrar passes manipulated `now_unix` to Registry, creating names with past or far-future expiry | High | Medium | Open | Both Registrar and Registry accept caller-supplied timestamps; no on-chain time cross-check at either layer |

### 3.3 NFT ↔ Registry divergence

| ID | Threat | Severity | Likelihood | Status | Notes |
|----|--------|----------|------------|--------|-------|
| X-06 | NFT ownership diverges from Registry ownership after name transfer | High | High | Open | NFT transfers are independent; Registry `transfer` does not update the NFT; NFT `transfer` does not update Registry. Downstream systems that rely on NFT ownership for identity decisions will be wrong. |
| X-07 | Anyone mints an NFT for a name they do not own in Registry | Critical | High | Open | `NFT.mint` has no auth and no Registry cross-check; phantom NFTs can be created for any name |
| X-08 | NFT for an expired or burned Registry name remains live | Medium | Medium | Open | Registry burn/expiry does not trigger NFT burn; stale NFTs persist and could mislead holders |

### 3.4 Subdomain ↔ Registry divergence

| ID | Threat | Severity | Likelihood | Status | Notes |
|----|--------|----------|------------|--------|-------|
| X-09 | Subdomain parent registered by non-owner of the Registry entry | Critical | High | Open | `register_parent` has no auth and no Registry ownership check; an attacker claiming the Subdomain parent namespace for `alice.xlm` blocks the real `alice.xlm` owner from using subdomains |
| X-10 | Registry entry for parent name expires; Subdomain parent record remains live, allowing continued subdomain creation | High | Medium | Open | No TTL sync between Subdomain and Registry; subdomain infrastructure persists after the parent name lapses |

### 3.5 Auction ↔ Registry divergence

| ID | Threat | Severity | Likelihood | Status | Notes |
|----|--------|----------|------------|--------|-------|
| X-11 | Auction created for an already-registered name; winning bidder receives nothing | High | High | Open | `create_auction` does not check Registry; a name might already have an owner and the winner has no mechanism to claim it |
| X-12 | Auction winner has no automatic mechanism to register the name in Registry | High | High | Open | Settlement transfers tokens but does not call Registry; the winner must separately register, which may fail if another party races to register first |

### 3.6 Caller-supplied `now_unix` across contracts

| ID | Threat | Severity | Likelihood | Status | Notes |
|----|--------|----------|------------|--------|-------|
| X-13 | Inconsistent `now_unix` values across a single logical operation create exploitable time windows | High | Medium | Open | Registry, Registrar, Resolver, Auction all accept caller-supplied timestamps; a caller who submits different values to different contracts in the same transaction can manufacture state inconsistencies |

---

## 4. State Divergence Summary

| Contracts | Shared State | Sync Mechanism | Divergence Risk |
|-----------|--------------|----------------|-----------------|
| Registry ↔ NFT | Name ownership | None | Critical |
| Registry ↔ Subdomain | Parent name active | None | Critical |
| Registry ↔ Auction | Name available for auction | None | High |
| Registry ↔ Resolver | Resolver ownership | Cross-contract call on every mutation | Low (mitigated) |
| Registrar ↔ Registry | Fee enforcement | Registry trusts Registrar to collect fees | High (no enforcement) |

---

## 5. Recommended Cross-Contract Mitigations

1. **NFT ↔ Registry sync**: NFT minting should be gated on Registry ownership verification.
   Either: (a) only the Registrar (which verifies Registry state) can call `NFT.mint`, or
   (b) `NFT.mint` cross-calls Registry to assert the token_id is a registered name owned
   by `owner`. Similarly, NFT transfer and Registry transfer should be atomically linked
   or the NFT contract should delegate ownership queries to the Registry.

2. **Subdomain ↔ Registry sync**: `register_parent` should cross-call Registry to verify
   that the caller is the current owner of the parent name and that the name is active.

3. **Auction ↔ Registry integration**: `create_auction` should verify the name is not
   currently registered (or is in a claimable state). Auction settlement should initiate
   or trigger Registry registration for the winner to give the payment meaning.

4. **Registrar as the sole `Registry.register` caller**: The Registry should store the
   authorized Registrar address and reject direct `register` calls from any other caller.
   This enforces fee collection at the protocol level.

5. **On-chain timestamp**: All contracts should clamp or validate caller-supplied
   `now_unix` against `env.ledger().timestamp()` with a tolerance. This eliminates an
   entire class of time-manipulation attacks across the system.

---

## 6. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
