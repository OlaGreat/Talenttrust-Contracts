# Escrow Contract Security Notes

This document describes the implemented escrow lifecycle, the contract's security boundaries, and the threat model reviewers should use when assessing changes in `contracts/escrow`.

## Contract scope

The current escrow contract implements four stateful behaviors:

- Create an escrow agreement with a fixed client, freelancer, and milestone schedule.
- Accept a single deposit that must exactly match the total of all milestones.
- Release milestones one time each until the contract reaches `Completed`.
- Allow a freelancer to write an informational reputation record only after a completed contract grants a pending reputation credit.

The contract does not yet move Stellar assets, integrate a dispute process, or provide arbitration. `Disputed` remains a reserved status for future work and is intentionally unreachable in the current state machine.

## Security invariants

- The client must authorize contract creation, funding, and milestone release.
- Client and freelancer addresses must be different.
- Each milestone amount must be strictly positive.
- Funding is all-or-nothing. Partial funding and overfunding are rejected.
- Milestones are immutable after creation and can only be released once.
- The sum of released milestones can never exceed the funded amount.
- Reputation updates require an earned credit from a completed contract and are consumed one-for-one.
- Reputation data is informational only and is not used to authorize fund movement.

## Threat model

### Unauthorized state transitions

Threat:
An attacker attempts to create, fund, or release another party's contract.

Mitigation:
The contract requires Soroban address authorization from the stored client for create, deposit, and release operations. Tests cover the missing-auth path.

### Underfunded or ambiguous escrow balances

Threat:
Partial deposits create uncertainty around which milestones are payable, or repeated deposits inflate the funded balance.

Mitigation:
`deposit_funds` accepts only one positive deposit whose amount exactly equals the fixed milestone total and only while the contract is in `Created`.

### Double release / replay of milestone payouts

Threat:
A milestone release is replayed, allowing the same work item to be paid more than once.

Mitigation:
Each milestone stores a `released` flag, release is irreversible, and the contract rejects repeated or out-of-range milestone indices.

### Accounting drift

Threat:
Stored totals become inconsistent and allow releases above the escrowed amount.

Mitigation:
The implementation stores total funded and total released amounts explicitly and rejects any release that would exceed the funded amount.

### Reputation abuse

Threat:
A freelancer writes reputation data without ever completing an escrow contract, or writes multiple ratings from a single completion.

Mitigation:
Completion of the final milestone creates exactly one pending reputation credit for the freelancer. `issue_reputation` consumes one credit per write and rejects out-of-range ratings.

Residual risk:
The current interface allows the freelancer to submit the rating value themselves because the method does not include a client or admin signer. The rating record is therefore documented as informational only. Future production work should bind rating issuance to a client-signed or protocol-signed attestation.

## Test mapping

- Lifecycle tests verify creation, full funding, milestone completion, and reputation issuance.
- Security tests cover auth failures, invalid milestones, partial or duplicate deposits, replayed releases, out-of-range milestone access, and invalid reputation attempts.

## Reviewer notes

- Any future token transfer integration must preserve the same state invariants and must be audited for atomicity across storage and transfer operations.
- If dispute resolution is added, `Disputed` transitions should explicitly freeze releases and define who is authorized to resolve the dispute.
