//! Per-identity nonce tracking for the delegation contract.
//!
//! Provides monotonically increasing nonces that bind each delegated-action
//! signature to a single use.  The same pattern used by `credence_bond::nonce`
//! is replicated here so the delegation contract remains self-contained.

use soroban_sdk::{Address, Env};

use crate::DataKey;

/// Returns the current nonce for `identity` (starts at 0).
///
/// Callers must supply this value in the next state-changing delegated call;
/// it is incremented on success.
#[must_use]
pub fn get_nonce(e: &Env, identity: &Address) -> u64 {
    e.storage()
        .instance()
        .get(&DataKey::Nonce(identity.clone()))
        .unwrap_or(0)
}

/// Asserts `expected_nonce` matches the stored nonce for `identity`, then
/// increments.  Panics on mismatch (replay or out-of-order submission).
pub fn consume_nonce(e: &Env, identity: &Address, expected_nonce: u64) {
    let current = get_nonce(e, identity);
    if current != expected_nonce {
        panic!("invalid nonce: replay or out-of-order");
    }
    let next = current.checked_add(1).expect("nonce overflow");
    e.storage()
        .instance()
        .set(&DataKey::Nonce(identity.clone()), &next);
}
