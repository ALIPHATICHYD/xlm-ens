# Name Lifecycle State Diagram

This diagram illustrates the complete lifecycle of a name in the xlm-ens system, from being unregistered to active, expired, and eventually available for auction.

```mermaid
stateDiagram-v2
    direction LR

    [*] --> Missing: Initial State
    Missing --> Active: register()
    
    state Active {
        direction LR
        [*] --> Resolving: Name is active
        Resolving --> Resolving: renew()
        Resolving --> Resolving: transfer()
        Resolving --> GracePeriod: expires_at
    }

    state GracePeriod {
        direction LR
        [*] --> Expired
        Expired --> Active: renew()
        Expired --> Claimable: grace_period_ends_at
    }

    state Claimable {
        direction LR
        [*] --> AuctionReady: Awaiting Bids
        AuctionReady --> Active: settle_auction()
    }

    Claimable --> Missing:誰も入札しない (No one bids)

```

## State Definitions & Transitions

1.  **Missing**
    *   **Description**: The name does not exist in the `Registry` and is available for general registration.
    *   **Available Actions**:
        *   `register()`: Anyone can register the name.
    *   **Transition**: On successful `register()` invocation, the state moves to `Active`.

2.  **Active**
    *   **Description**: The name is owned, active, and can be resolved to its target records. The owner has full control.
    *   **Available Actions**:
        *   `renew()`: Extend the registration period.
        *   `transfer()`: Change the owner.
        *   `resolve()`: Query its records.
    *   **Transition**: When the current timestamp (`now`) exceeds `expires_at`, the name transitions to the `GracePeriod`.

3.  **GracePeriod**
    *   **Description**: The name has expired but is protected for a fixed duration (default: 90 days), during which only the previous owner can renew it. The name no longer resolves.
    *   **Available Actions**:
        *   `renew()`: The original owner can reclaim the name by paying a renewal fee.
    *   **Transition**:
        *   If renewed, it returns to the `Active` state.
        *   If the grace period ends (`grace_period_ends_at` is reached), it transitions to `Claimable`.

4.  **Claimable**
    *   **Description**: The name has passed its grace period and is now eligible for a public auction.
    *   **Available Actions**:
        *   `bid()`: Anyone can place a bid in the `Auction` contract.
    *   **Transition**:
        *   If an auction is settled via `settle_auction()`, the winner becomes the new owner, and the state returns to `Active`.
        *   If no bids are placed and the auction period closes, the name may be returned to the `Missing` state (or handled by a specific purging mechanism).

## Timing Parameters

These durations are configurable via governance proposals.

| Parameter             | Default   | Description                                                                  |
| --------------------- | --------- | ---------------------------------------------------------------------------- |
| `registration_years`  | 1-5 years | Duration selected by the user during registration.                           |
| `grace_period_duration` | 90 days   | Time after expiry during which only the owner can renew.                     |
| `auction_duration`      | 7 days    | The period during which a name in the `Claimable` state can be bid on.       |
