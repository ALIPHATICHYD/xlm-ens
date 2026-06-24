# Threat Model: Subdomain Contract

**Status:** Draft  
**Contract:** `contracts/subdomain/src/lib.rs`  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

---

## 1. Contract Overview

The Subdomain contract manages the namespace below registered `.xlm` names. A parent-domain
owner registers a parent record and then delegates creation rights to one or more
controllers. Controllers can create, delete, and revoke subdomains within the parent
namespace. Subdomains are independent records with their own owner and can be transferred.
A namespace integrity failure here allows attackers to create illegitimate subdomains under
any parent, hijack the parent namespace, or evict legitimate subdomain holders.

---

## 2. Asset Inventory

| Asset | Description | Integrity | Availability |
|-------|-------------|-----------|--------------|
| Parent domain records | Owner and controller list per parent | Critical | High |
| Subdomain records | FQDN → owner, parent, created_at | Critical | High |
| Parent subdomain index | Per-parent list of child FQDNs | High | Medium |
| Owner subdomain index | Per-address list of owned FQDNs | High | Medium |
| Admin key | Controls upgrades | Critical | High |

---

## 3. Trust Boundaries and Privilege Model

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | `upgrade` | `admin.require_auth()` |
| Parent owner | `register_parent`, `add_controller`, `remove_controller`, `create`, `delete`, `revoke` | Equality check only — **missing `caller.require_auth()`** |
| Controller | `create`, `delete`, `revoke` | Equality check only — **missing `caller.require_auth()`** |
| Subdomain owner | `transfer`, `delete` (own record) | Equality check only — **missing `caller.require_auth()`** |
| Anyone | `register_parent` | **None — no auth at all (see Open Issues)** |
| Public | `exists`, `parent`, `record`, `subdomains_for_parent`, `subdomains_for_owner` | None |

---

## 4. External Dependencies

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| `xlm_ns_common` validation | Compile-time | Invalid FQDNs or base names accepted |
| Registry contract | Not called — subdomain ownership is independent | Parent-domain Registry ownership not verified |
| Soroban runtime | Critical | Storage and auth primitives |

---

## 5. STRIDE Threat Analysis

### 5.1 Spoofing

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-SUB-01 | Attacker registers any parent domain with themselves as owner without owning the Registry name | Critical | High | **Open** | `register_parent` has no auth; no check against Registry ownership |
| S-SUB-02 | Attacker supplies `caller=<parent_owner>` to `add_controller`/`remove_controller` without signature | Critical | High | **Open** | Equality check `parent_record.owner != caller` with no `caller.require_auth()` |
| S-SUB-03 | Attacker supplies `caller=<owner>` to `transfer`, `delete`, `revoke` without signature | Critical | High | **Open** | Equality checks throughout but no `caller.require_auth()` anywhere in the contract |
| S-SUB-04 | Attacker impersonates admin to upgrade | Critical | Low | Mitigated | `admin.require_auth()` enforced in `upgrade` |

### 5.2 Tampering

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-SUB-01 | Attacker claims any parent namespace (e.g. `alice.xlm`) by calling `register_parent` first | Critical | High | **Open** | No auth, no Registry ownership check; first caller wins |
| T-SUB-02 | Attacker adds themselves as controller of any parent to gain creation/deletion rights | Critical | High | **Open** | Follows from S-SUB-02; no `require_auth()` on `add_controller` |
| T-SUB-03 | Attacker creates arbitrary subdomains under any parent (e.g. `fake.alice.xlm`) | Critical | High | **Open** | Follows from T-SUB-01 and T-SUB-02 |
| T-SUB-04 | Attacker transfers subdomain ownership to themselves without current owner's consent | Critical | High | **Open** | Follows from S-SUB-03; no `require_auth()` on `transfer` |
| T-SUB-05 | Subdomain records lack TTL extension; entries silently age out | Medium | Low | Open | No `extend_ttl` calls in subdomain contract; persistent entries expire |

### 5.3 Repudiation

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-SUB-01 | Parent registration event attributes unverified `owner` address | High | High | **Open** | Event `(parent, record.owner.clone())` emitted but `owner` param is caller-supplied with no auth |
| R-SUB-02 | Subdomain creation event attributes unverified `caller` | High | High | **Open** | Event emitted for `create`/`delete`/`revoke` but caller is not verified |

### 5.4 Information Disclosure

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-SUB-01 | Controller list for any parent is publicly enumerable | Low | High | Accepted | By design; controller addresses are on-chain |
| I-SUB-02 | `subdomains_for_owner` reveals all subdomains owned by an address | Low | High | Accepted | By design |

### 5.5 Denial of Service

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-SUB-01 | Attacker registers every possible base name as parent, blocking real owners | Critical | High | **Open** | No auth on `register_parent`; names are first-come-first-served |
| D-SUB-02 | Attacker as fraudulent parent owner revokes all subdomains | High | High | **Open** | Follows from T-SUB-01; fraudulent parent owner can call `revoke` on all children |
| D-SUB-03 | `subdomains_for_parent`/`subdomains_for_owner` return unbounded lists | Medium | Medium | Open | No pagination cap; large subdomain counts cause unbounded return values |
| D-SUB-04 | Persistent entries silently expire without TTL extension | Medium | Low | Open | No `extend_ttl` calls; entries age out after ~3 months by default |

### 5.6 Elevation of Privilege

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-SUB-01 | Attacker takes ownership of parent namespace without owning the Registry name | Critical | High | **Open** | Root cause: T-SUB-01; controls creation and deletion of all subdomains |
| E-SUB-02 | Controller escalates to own all subdomains by transferring them to themselves | High | High | **Open** | Follows from S-SUB-03; controllers can call `transfer` impersonating subdomain owners |

---

## 6. Attack Surface Summary

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | Deployment race |
| `upgrade` | Admin | `admin.require_auth()` | Correctly gated |
| `register_parent` | **Anyone** | **None — no auth** | **CRITICAL — T-SUB-01, D-SUB-01** |
| `add_controller` | Parent owner | Equality check — **no `require_auth()`** | **CRITICAL — S-SUB-02** |
| `remove_controller` | Parent owner | Equality check — **no `require_auth()`** | **CRITICAL — S-SUB-02** |
| `create` | Parent owner or controller | Equality check — **no `require_auth()`** | **CRITICAL — S-SUB-03** |
| `transfer` | Subdomain owner | Equality check — **no `require_auth()`** | **CRITICAL — S-SUB-03** |
| `delete` | Subdomain owner / parent owner / controller | Equality check — **no `require_auth()`** | **CRITICAL — S-SUB-03** |
| `revoke` | Subdomain owner / parent owner / controller | Equality check — **no `require_auth()`** | **CRITICAL — S-SUB-03** |
| `exists` | Public | None | Read-only |
| `parent` | Public | None | Read-only |
| `record` | Public | None | Read-only |
| `subdomains_for_parent` | Public | None | Unbounded |
| `subdomains_for_owner` | Public | None | Unbounded |

---

## 7. Open Issues

1. **S-SUB-01 / T-SUB-01 / D-SUB-01 / E-SUB-01 — `register_parent` unauthenticated**:
   Anyone can claim any parent namespace. The caller-supplied `owner` is stored as-is
   without verifying that the caller owns the corresponding Registry entry. Fix: require
   the `owner` to sign (`owner.require_auth()`) AND verify `owner` matches the Registry
   entry for `parent` via a cross-contract call.

2. **S-SUB-02 / T-SUB-02 — `add_controller`/`remove_controller` missing
   `caller.require_auth()`**: Equality check against stored owner is performed but
   `caller` is never asked to sign. Fix: add `caller.require_auth()` to both functions.

3. **S-SUB-03 / T-SUB-03 / T-SUB-04 / E-SUB-02 — `create`, `transfer`, `delete`,
   `revoke` missing `caller.require_auth()`**: All four mutating subdomain functions
   check role membership (owner/controller) but never verify the caller's signature.
   Fix: add `caller.require_auth()` at the top of each function.

4. **T-SUB-05 / D-SUB-04 — No TTL extension**: Parent and subdomain records use
   persistent storage but no `extend_ttl` call is made on write. Records will expire
   after the network default. Add `extend_ttl` analogous to the resolver's
   `extend_persistent_ttl` pattern.

5. **D-SUB-03 — Unbounded `subdomains_for_parent`/`subdomains_for_owner`**: Add
   offset/limit pagination parameters and cap at a maximum (e.g. 200).

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
