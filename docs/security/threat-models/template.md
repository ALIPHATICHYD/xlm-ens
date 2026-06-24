# Threat Model Template

**Status:** Active  
**Framework:** STRIDE  
**Last Updated:** 2026-06-24

This template defines the structure every contract-level threat model must follow.
Copy it verbatim and fill in each section. Do not omit sections — write "None identified"
where a category does not apply.

---

## 1. Contract Overview

Describe the contract in 2–4 sentences: what it does, what assets it protects, and what
system invariants it must uphold.

---

## 2. Asset Inventory

List every asset the contract is responsible for protecting.

| Asset | Description | Confidentiality | Integrity | Availability |
|-------|-------------|-----------------|-----------|--------------|
| Example: Admin key | Controls upgrades and governance | High | Critical | High |

---

## 3. Trust Boundaries and Privilege Model

List every caller role and what actions each is permitted to perform.

| Role | Permitted Actions | Auth Mechanism |
|------|-------------------|----------------|
| Admin | Upgrade, governance | `require_auth()` on stored admin address |
| Owner | Mutate own records | `require_auth()` on owner address |
| Public | Read-only queries | None |

---

## 4. External Dependencies

List every contract, token, or system component this contract calls or trusts.

| Dependency | Trust Level | Risk if Compromised |
|------------|-------------|---------------------|
| Registry contract | High — ownership decisions delegated | Attacker can spoof ownership |

---

## 5. STRIDE Threat Analysis

For each threat category, enumerate identified threats, their risk rating, and status.

**Risk Rating Scale**

| Severity | Likelihood | Rating |
|----------|------------|--------|
| Critical + High | High | Critical |
| High + Medium | Medium | High |
| Medium + Low | Low | Medium |
| Low + any | any | Low |

**Status values:** Open · Mitigated · Accepted

### 5.1 Spoofing

Threats where an attacker impersonates a legitimate actor.

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| S-XX-01 | Description | High | Medium | Mitigated | How it is addressed |

### 5.2 Tampering

Threats where an attacker modifies state or data without authorization.

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| T-XX-01 | Description | High | Medium | Open | Proposed fix |

### 5.3 Repudiation

Threats where an actor denies performing an action.

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| R-XX-01 | Description | Medium | Low | Mitigated | Events emitted on all state changes |

### 5.4 Information Disclosure

Threats where sensitive information is exposed to unauthorized parties.

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| I-XX-01 | Description | Medium | Low | Accepted | All storage is public on-chain |

### 5.5 Denial of Service

Threats that prevent legitimate use of the contract.

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| D-XX-01 | Description | Medium | Medium | Open | Proposed fix |

### 5.6 Elevation of Privilege

Threats where an actor gains capabilities beyond their role.

| ID | Threat | Severity | Likelihood | Status | Mitigation |
|----|--------|----------|------------|--------|------------|
| E-XX-01 | Description | Critical | Low | Open | Proposed fix |

---

## 6. Attack Surface Summary

Enumerate all public entry points and their exposure.

| Function | Caller | Auth Required | Risk Notes |
|----------|--------|---------------|------------|
| `initialize` | Deployer (once) | None — blocked on re-call | First-call race window |
| `upgrade` | Admin | `require_auth()` | Admin key compromise |

---

## 7. Open Issues

List threats with status Open, together with recommended remediation.

1. **[ID] Title** — Description and recommended fix.

---

## 8. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-24 | | Initial draft |
