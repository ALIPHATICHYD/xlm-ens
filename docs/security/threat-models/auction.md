# Threat Model: Auction Contract

**Status:** Draft  
**Contract:** `contracts/auction/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The Auction contract implements a Vickrey (second-price, sealed-bid) auction mechanism
for `.xlm` names. Bidders lock tokens during the auction window; the highest bidder wins
at the second-highest price, with all other bidders refunded on settlement. The contract
holds real token balances in escrow and routes payments to a treasury address at
settlement. Any auction-related token loss or manipulation is a direct financial loss.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Escrowed bid tokens | Sum of all unreturned bids in contract custody | Critical | High |
| Auction records | `name`, `reserve_price`, `starts_at`, `ends_at`, `bids`, `asset`, `treasury` | Critical | High |
| Settlement records | Winner, clearing price, sold flag | High | High |
| Treasury address per auction | Receives clearing price on settlement | Critical | High |
| Auction names index | Discovery index used for pagination | Medium | Medium |
| Admin key | Controls upgrades | High | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `admin.require_auth()` |
| Bidder | `place_bid` | `bidder.require_auth()` |
| Anyone | `create_auction` | **None — CRITICAL BUG (see Open Issues)** |
| Anyone | `settle` (after end time) | None — intentional; permissionless settlement |
| Public | All read functions | None |

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| Token contract (`auction.asset`) | Critical — holds and transfers escrow | Malicious or broken token can drain bids or block refunds |
| Treasury address | High — receives clearing price | Malicious treasury contract could block settlement |
| Soroban runtime | Critical | Storage, auth, and token call integrity |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-AUC-01 | Attacker impersonates bidder to place bid debiting victim's token balance | Critical | Low | Mitigated | `bidder.require_auth()` enforced in `place_bid`; token transfer requires bidder approval |
| S-AUC-02 | Attacker impersonates admin to upgrade contract | Critical | Low | Mitigated | `admin.require_auth()` enforced in `upgrade` |
| S-AUC-03 | Attacker creates a fraudulent auction with a spoofed `treasury` address pointing to themselves | High | High | **Open** | No auth on `create_auction`; attacker can set `treasury=attacker` and drain clearing price |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-AUC-01 | Attacker creates auction for a name they do not own, locking out the real owner | High | High | **Open** | No auth on `create_auction`; name ownership in Registry is not verified |
| T-AUC-02 | Attacker creates auction with `reserve_price=0` and `ends_at=starts_at`, then immediately settles | High | High | **Open** | No validation of auction timing or reserve floor; attacker gets name for 0 cost |
| T-AUC-03 | Attacker supplies past `now_unix` to `place_bid` to bid outside the real time window | Medium | Medium | Open | `now_unix` is caller-supplied; `is_time_window_open` uses caller's timestamp |
| T-AUC-04 | Attacker supplies future `now_unix` to `settle` to settle before auction end | Medium | Medium | Open | `settle` checks `now_unix < auction.ends_at`; caller supplies `now_unix` |
| T-AUC-05 | Vickrey algorithm gives win to first highest bidder; second bidder at same price is disadvantaged | Low | Low | Accepted | Deterministic tie-breaking by insertion order; documented behavior |
| T-AUC-06 | Attacker creates auction with malicious `asset` token contract | Critical | Medium | **Open** | No auth on `create_auction`; `asset` is arbitrary; re-entrant or honeypot token could trap bids |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-AUC-01 | Auction created by unauthorized party with no event identifying creator | Medium | High | **Open** | No auth on `create_auction`; no event emitted from `create_auction`; creator is untracked |
| R-AUC-02 | Settlement disputed with no on-chain attribution of settler | Low | Low | Accepted | `Settlement` struct records winner/price/time; caller of `settle` is not attributed but is permissionless |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-AUC-01 | All bids are public; last-second snipers can observe all prior bids | Medium | High | Accepted | Vickrey auction is designed to be sealed but on-chain bids are readable; this is a known limitation of on-chain auctions |
| I-AUC-02 | `auction.treasury` address is readable by anyone before settlement | Low | High | Accepted | All storage is public; no secret here |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-AUC-01 | Attacker creates thousands of dummy auctions filling the names index | Medium | Medium | Open | No auth on `create_auction`; names index is append-only and unbounded; pagination is capped at 100 but index storage grows |
| D-AUC-02 | Treasury contract reverts on token transfer, permanently blocking settlement | High | Low | Open | If `treasury` is a contract that panics on receive, the `settle` call will always fail; bids locked forever |
| D-AUC-03 | Asset token with transfer fee breaks refund logic (refund amount ≠ bid amount) | High | Low | Open | No auth on `create_auction`; custom tokens can have fee-on-transfer behavior causing refund shortfalls |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-AUC-01 | Attacker creates auction with `treasury=attacker` to steal all clearing-price payments | Critical | High | **Open** | No auth on `create_auction`; no restriction on `treasury` address |
| E-AUC-02 | Attacker bids on their own auction to set a controlled clearing price | Medium | High | Open | Allowed by design but creates wash-bidding; no self-bid restriction |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | Deployment race |
| `upgrade` | Admin | `admin.require_auth()` | Correctly gated |
| `create_auction` | **Anyone** | **None** | **CRITICAL — T-AUC-01, T-AUC-06, E-AUC-01** |
| `place_bid` | Bidder | `bidder.require_auth()` + token transfer | Correctly gated |
| `settle` | Anyone (after end) | None — permissionless | Caller-supplied `now_unix` |
| `auction` | Public | None | Read-only |
| `auction_names` | Public | None | Bounded at 100 |
| `active_auctions` | Public | None | Bounded at 100 |
| `settled_auctions` | Public | None | Bounded at 100 |
| `list_auctions` | Public | None | Paginated |
| `list_active_auctions` | Public | None | Paginated |
| `list_settled_auctions` | Public | None | Paginated |
| `auction_count` | Public | None | Read-only |

---

## 7. Open Issues

1. **T-AUC-01 / S-AUC-03 / E-AUC-01 — `create_auction` unauthenticated**: Anyone can
   create an auction for any name with arbitrary `treasury`, `asset`, `reserve_price`, and
   timing. This enables: redirecting clearing-price payments to an attacker, locking a
   legitimate owner's name in an auction, and deploying honeypot token contracts as the
   auction asset. Add `admin.require_auth()` or require the name's Registry owner to sign.

2. **T-AUC-06 — Arbitrary `asset` token contract**: A malicious token passed as `asset`
   can re-enter the auction on `transfer`, block refunds, or steal bids. Maintain an
   allowlist of approved token contracts that can be used as auction assets.

3. **D-AUC-02 — Treasury reverting blocks settlement permanently**: If the treasury
   contract reverts, all bids are locked in escrow indefinitely. Add a fallback withdrawal
   path for bids when settlement fails, or validate `treasury` against a known-safe list.

4. **T-AUC-03 / T-AUC-04 — Caller-supplied `now_unix`**: Both `place_bid` and `settle`
   use caller-supplied time for window checks. Validate against `env.ledger().timestamp()`
   with a reasonable tolerance.

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
