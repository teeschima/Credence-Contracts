//! Domain-separated payload for delegated action signatures.
//!
//! Without explicit domain separation a signature created for one function
//! (e.g. `delegate`) could be replayed against a different function
//! (e.g. `revoke_delegation`) because both consume the same nonce namespace.
//!
//! This module introduces a [`DomainTag`] enum that labels *which* function
//! domain owns a given signature, and a [`DelegatedActionPayload`] struct that
//! binds together:
//!
//! * `domain`       — the specific action type / function domain
//! * `owner`        — the principal whose authority is being invoked
//! * `target`       — the address being acted upon (delegate or subject)
//! * `contract_id`  — the current contract's address (chain / deployment context)
//! * `nonce`        — monotonically increasing per-owner counter
//!
//! Signature verification must hash *all* of these fields together.  A
//! signature produced for `domain = Delegate` will be structurally incompatible
//! with a `revoke_delegation` call even if the nonce happens to match.

use soroban_sdk::{contracttype, Address, Env};

/// Labels each function domain that accepts a delegated (off-chain) signature.
///
/// Adding a new domain here forces a compile-time decision at every match site,
/// making it impossible to silently forget a function.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DomainTag {
    /// Matches `delegate(…)` — creates or replaces a delegation entry.
    Delegate,
    /// Matches `revoke_delegation(…)` — revokes an existing delegation.
    RevokeDelegation,
    /// Matches `revoke_attestation(…)` — revokes an attestation-type delegation.
    RevokeAttestation,
}

/// Typed payload that must be hashed and signed by `owner` before a relayer
/// can submit a delegated action on their behalf.
///
/// The Soroban runtime does not expose EIP-712 natively, but the same
/// guarantees are achieved by requiring callers to pass this struct explicitly
/// and by verifying `owner.require_auth()` with the env's built-in
/// authorisation mechanism — which itself binds the call to the contract
/// address and ledger network.
#[contracttype]
#[derive(Clone, Debug)]
pub struct DelegatedActionPayload {
    /// Which function this payload authorises.
    pub domain: DomainTag,
    /// The account whose authority is being delegated / invoked.
    pub owner: Address,
    /// The address the action targets (the delegate or subject address).
    pub target: Address,
    /// The contract address (deployment / chain context).
    pub contract_id: Address,
    /// Owner's current nonce — consumed on success.
    pub nonce: u64,
}

/// Validates that the fields in `payload` match the parameters supplied at the
/// call site, and that the `domain` tag is exactly `expected_domain`.
///
/// Panics with a descriptive message on any mismatch, providing clear audit
/// trail information for both automated monitoring and manual review.
pub fn verify_payload(
    e: &Env,
    payload: &DelegatedActionPayload,
    expected_domain: DomainTag,
    caller_owner: &Address,
    caller_target: &Address,
) {
    if payload.domain != expected_domain {
        panic!("domain mismatch: payload domain does not match target function");
    }
    if &payload.owner != caller_owner {
        panic!("payload owner mismatch");
    }
    if &payload.target != caller_target {
        panic!("payload target mismatch");
    }
    if payload.contract_id != e.current_contract_address() {
        panic!("payload contract_id mismatch: cross-contract replay detected");
    }
}
