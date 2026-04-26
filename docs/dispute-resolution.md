# Dispute Resolution: Closure Invariants

This document describes terminal-state closure guarantees for the `dispute_resolution` contract.

## Closure API

- `resolve_dispute(closer: Address, dispute_id: u64)`
- `expire_dispute(closer: Address, dispute_id: u64)`

Both closure functions require explicit signer authorization from `closer` and enforce role checks.

## Authorized Closers

A dispute may be closed only by:

- the original `disputer`, or
- the contract `admin` (when initialized)

Any other caller is rejected with:

- `Error::Unauthorized` (`#6`)

## Terminal-State Invariants

The contract enforces deterministic terminal state:

- No double-close: disputes in `Resolved` or `Expired` cannot be closed again (`Error::DisputeNotOpen`, `#3`)
- No unauthorized close: closure attempts by non-disputer/non-admin fail (`Error::Unauthorized`, `#6`)

These checks ensure a dispute transitions from `Open` to exactly one terminal state and remains immutable afterward.

## Regression Coverage

Tests in `contracts/dispute_resolution/src/test.rs` cover:

- unauthorized resolve attempts
- unauthorized expire attempts
- state immutability after failed second close
