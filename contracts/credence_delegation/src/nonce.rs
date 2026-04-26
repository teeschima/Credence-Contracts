//! Per-identity nonce tracking for the delegation contract.
//!
//! Provides monotonically increasing nonces that bind each delegated-action
//! signature to a single use.  The same pattern used by `credence_bond::nonce`
//! is replicated here so the delegation contract remains self-contained.

use soroban_sdk::{Address, Env};
use soroban_sdk::panic_with_error;
use credence_errors::ContractError;

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
        panic_with_error!(e, ContractError::InvalidNonce);
    }
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(e, ContractError::Overflow));
    e.storage()
        .instance()
        .set(&DataKey::Nonce(identity.clone()), &next);
}

/// Advances nonce to `new_nonce`, invalidating the half-open range
/// `[current_nonce, new_nonce)`.
///
/// This allows compromised-key recovery by skipping potentially leaked,
/// pre-signed delegated payloads without submitting each nonce one-by-one.
///
/// # Panics
/// Panics if `new_nonce <= current_nonce` or if the span exceeds `max_span`.
pub fn invalidate_nonce_range(
    e: &Env,
    identity: &Address,
    new_nonce: u64,
    max_span: u64,
) -> (u64, u64) {
    let current = get_nonce(e, identity);
    if new_nonce <= current {
        panic_with_error!(e, ContractError::InvalidNonce);
    }
    let span = new_nonce
        .checked_sub(current)
        .unwrap_or_else(|| panic_with_error!(e, ContractError::Underflow));
    if span > max_span {
        panic_with_error!(e, ContractError::InvalidNonce);
    }

    e.storage()
        .instance()
        .set(&DataKey::Nonce(identity.clone()), &new_nonce);
    (current, new_nonce)
}
