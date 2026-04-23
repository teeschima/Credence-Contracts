//! # Pull-Payment Claims Module
//!
//! Implements a pull-payment pattern for reward claims to prevent griefing attacks
//! and failed transfers due to recipient contract fallback behavior.
//!
//! ## Features
//! - Pending claims tracking per user
//! - Batch claim processing
//! - Claim expiry mechanism
//! - Comprehensive event emission
//! - Gas-optimized claim operations

use crate::{events, DataKey};
use soroban_sdk::{contracttype, Address, Env, Map, Symbol, Vec};

/// Maximum number of claims that can be processed in a single batch
const MAX_BATCH_CLAIMS: u32 = 50;

/// Default claim expiry period (30 days in seconds)
const DEFAULT_CLAIM_EXPIRY: u64 = 30 * 24 * 60 * 60;

/// Types of claimable rewards
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimType {
    /// Verifier rewards from successful attestations
    VerifierReward = 0,
    /// Slashing rewards for successful challenges
    SlashingReward = 1,
    /// Early exit penalty refunds (partial)
    PenaltyRefund = 2,
    /// Protocol fee rebates
    FeeRebate = 3,
    /// Dispute resolution rewards
    DisputeReward = 4,
}

/// A pending claim for a user
#[contracttype]
#[derive(Clone, Debug)]
pub struct PendingClaim {
    /// Unique claim ID
    pub claim_id: u64,
    /// Type of claim
    pub claim_type: ClaimType,
    /// Amount to be claimed
    pub amount: i128,
    /// When the claim was created
    pub created_at: u64,
    /// When the claim expires (0 = no expiry)
    pub expires_at: u64,
    /// Source transaction or event that generated this claim
    pub source_id: u64,
    /// Additional metadata (optional)
    pub metadata: Symbol,
    /// Whether this claim has been processed
    pub processed: bool,
}

/// Result of a claim operation
#[contracttype]
#[derive(Clone, Debug)]
pub struct ClaimResult {
    /// Number of claims processed
    pub processed_count: u32,
    /// Total amount claimed
    pub total_amount: i128,
    /// List of claim types processed
    pub claim_types: Vec<ClaimType>,
}

/// Storage keys for claims
impl DataKey {
    /// Pending claims for a user: DataKey::PendingClaims(user) -> Vec<PendingClaim>
    pub fn pending_claims(user: &Address) -> DataKey {
        DataKey::PendingClaims(user.clone())
    }

    /// Total claimable amount for a user: DataKey::ClaimableAmount(user) -> i128
    pub fn claimable_amount(user: &Address) -> DataKey {
        DataKey::ClaimableAmount(user.clone())
    }

    /// Claim history counter: DataKey::ClaimCounter -> u64
    pub const fn claim_counter() -> DataKey {
        DataKey::ClaimCounter
    }

    /// Individual claim by ID: DataKey::ClaimById(claim_id) -> PendingClaim
    pub const fn claim_by_id(claim_id: u64) -> DataKey {
        DataKey::ClaimById(claim_id)
    }
}

/// Add a new pending claim for a user
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address of the user who can claim
/// * `claim_type` - Type of claim being added
/// * `amount` - Amount to be claimed
/// * `source_id` - Source transaction/event ID
/// * `metadata` - Optional metadata
///
/// # Panics
/// * If amount is negative or zero
pub fn add_pending_claim(
    e: &Env,
    user: &Address,
    claim_type: ClaimType,
    amount: i128,
    source_id: u64,
    metadata: Option<Symbol>,
) -> u64 {
    if amount <= 0 {
        panic!("claim amount must be positive");
    }

    // Get next claim ID
    let claim_id = get_next_claim_id(e);

    let now = e.ledger().timestamp();
    let expires_at = now + DEFAULT_CLAIM_EXPIRY;

    let claim = PendingClaim {
        claim_id,
        claim_type,
        amount,
        created_at: now,
        expires_at,
        source_id,
        metadata: metadata.unwrap_or(Symbol::new(e, "")),
        processed: false,
    };

    // Store claim by ID for direct access
    e.storage()
        .persistent()
        .set(&DataKey::ClaimById(claim_id), &claim.clone());

    // Get existing claims or create new vector
    let mut claims: Vec<PendingClaim> = e
        .storage()
        .persistent()
        .get(&DataKey::PendingClaims(user.clone()))
        .unwrap_or(Vec::new(e));

    claims.push_back(claim.clone());

    // Update storage
    e.storage()
        .persistent()
        .set(&DataKey::PendingClaims(user.clone()), &claims);

    // Update total claimable amount
    let current_total: i128 = e
        .storage()
        .persistent()
        .get(&DataKey::ClaimableAmount(user.clone()))
        .unwrap_or(0);

    let new_total = current_total
        .checked_add(amount)
        .expect("claimable amount overflow");

    e.storage()
        .persistent()
        .set(&DataKey::ClaimableAmount(user.clone()), &new_total);

    // Emit event
    events::emit_claim_added(e, user, &claim);

    claim_id
}

/// Get the next claim ID
fn get_next_claim_id(e: &Env) -> u64 {
    let current: u64 = e
        .storage()
        .persistent()
        .get(&DataKey::ClaimCounter)
        .unwrap_or(0);
    let next = current.checked_add(1).expect("claim counter overflow");
    e.storage().persistent().set(&DataKey::ClaimCounter, &next);
    next
}

/// Get all pending claims for a user
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address to check claims for
///
/// # Returns
/// Vector of pending claims (empty if none)
pub fn get_pending_claims(e: &Env, user: &Address) -> Vec<PendingClaim> {
    e.storage()
        .persistent()
        .get(&DataKey::PendingClaims(user.clone()))
        .unwrap_or(Vec::new(e))
}

/// Get total claimable amount for a user
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address to check amount for
///
/// # Returns
/// Total amount that can be claimed
pub fn get_claimable_amount(e: &Env, user: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::ClaimableAmount(user.clone()))
        .unwrap_or(0)
}

/// Process claims for a user (pull-payment pattern)
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address claiming rewards
/// * `claim_types` - Optional filter for specific claim types (empty = all types)
/// * `max_claims` - Maximum number of claims to process (0 = no limit, capped at MAX_BATCH_CLAIMS)
///
/// # Returns
/// ClaimResult with details of processed claims
///
/// # Panics
/// * If user has no pending claims
/// * If token transfer fails
pub fn process_claims(
    e: &Env,
    user: &Address,
    claim_types: Vec<ClaimType>,
    max_claims: u32,
) -> ClaimResult {
    user.require_auth();

    let now = e.ledger().timestamp();
    let mut claims = get_pending_claims(e, user);

    if claims.is_empty() {
        panic!("no pending claims");
    }

    // Filter claims by type if specified
    let filter_types = !claim_types.is_empty();
    let type_set: Map<ClaimType, bool> = if filter_types {
        let mut map = Map::new(e);
        for i in 0..claim_types.len() {
            map.set(claim_types.get(i).unwrap(), true);
        }
        map
    } else {
        Map::new(e)
    };

    let mut processed_claims = Vec::new(e);
    let mut remaining_claims = Vec::new(e);
    let mut total_amount = 0i128;
    let mut processed_types = Vec::new(e);
    let limit = if max_claims == 0 {
        MAX_BATCH_CLAIMS
    } else {
        max_claims.min(MAX_BATCH_CLAIMS)
    };

    // Process claims
    for i in 0..claims.len() {
        if processed_claims.len() >= limit {
            // Add remaining claims back to the list
            for j in i..claims.len() {
                remaining_claims.push_back(claims.get(j).unwrap());
            }
            break;
        }

        let claim = claims.get(i).unwrap();

        // Skip expired claims
        if claim.expires_at > 0 && now > claim.expires_at {
            continue;
        }

        // Skip if not in filter
        if filter_types && !type_set.contains_key(claim.claim_type) {
            remaining_claims.push_back(claim);
            continue;
        }

        // Process this claim
        processed_claims.push_back(claim.clone());
        total_amount = total_amount
            .checked_add(claim.amount)
            .expect("claim total overflow");

        // Track unique claim types
        let mut type_exists = false;
        for j in 0..processed_types.len() {
            if processed_types.get(j).unwrap() == claim.claim_type {
                type_exists = true;
                break;
            }
        }
        if !type_exists {
            processed_types.push_back(claim.claim_type);
        }
    }

    if processed_claims.is_empty() {
        panic!("no valid claims to process");
    }

    // Update storage with remaining claims
    if remaining_claims.is_empty() {
        e.storage()
            .persistent()
            .remove(&DataKey::PendingClaims(user.clone()));
        e.storage()
            .persistent()
            .remove(&DataKey::ClaimableAmount(user.clone()));
    } else {
        e.storage()
            .persistent()
            .set(&DataKey::PendingClaims(user.clone()), &remaining_claims);

        let remaining_amount = get_claimable_amount(e, user)
            .checked_sub(total_amount)
            .expect("claimable amount underflow");

        e.storage()
            .persistent()
            .set(&DataKey::ClaimableAmount(user.clone()), &remaining_amount);
    }

    // Transfer tokens to user
    if total_amount > 0 {
        let token: Address = e
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("token not configured");

        let contract = e.current_contract_address();
        soroban_sdk::token::TokenClient::new(e, &token).transfer(&contract, user, &total_amount);
    }

    let result = ClaimResult {
        processed_count: processed_claims.len(),
        total_amount,
        claim_types: processed_types,
    };

    // Emit events
    events::emit_claims_processed(e, user, &result, &processed_claims);

    result
}

/// Clean up expired claims for a user (can be called by anyone for gas efficiency)
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address to clean up claims for
///
/// # Returns
/// Number of expired claims removed
pub fn cleanup_expired_claims(e: &Env, user: &Address) -> u32 {
    let now = e.ledger().timestamp();
    let claims = get_pending_claims(e, user);

    if claims.is_empty() {
        return 0;
    }

    let mut valid_claims = Vec::new(e);
    let mut expired_amount = 0i128;
    let mut expired_count = 0u32;

    for i in 0..claims.len() {
        let claim = claims.get(i).unwrap();

        if claim.expires_at > 0 && now > claim.expires_at {
            expired_amount = expired_amount
                .checked_add(claim.amount)
                .expect("expired amount overflow");
            expired_count += 1;
        } else {
            valid_claims.push_back(claim);
        }
    }

    if expired_count > 0 {
        // Update storage
        if valid_claims.is_empty() {
            e.storage()
                .persistent()
                .remove(&DataKey::PendingClaims(user.clone()));
            e.storage()
                .persistent()
                .remove(&DataKey::ClaimableAmount(user.clone()));
        } else {
            e.storage()
                .persistent()
                .set(&DataKey::PendingClaims(user.clone()), &valid_claims);

            let remaining_amount = get_claimable_amount(e, user)
                .checked_sub(expired_amount)
                .expect("claimable amount underflow");

            e.storage()
                .persistent()
                .set(&DataKey::ClaimableAmount(user.clone()), &remaining_amount);
        }

        // Emit event
        events::emit_claims_expired(e, user, expired_count, expired_amount);
    }

    expired_count
}

/// Get claims summary for a user
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address to get summary for
///
/// # Returns
/// Map of claim types to total amounts
pub fn get_claims_summary(e: &Env, user: &Address) -> Map<ClaimType, i128> {
    let claims = get_pending_claims(e, user);
    let mut summary = Map::new(e);

    for i in 0..claims.len() {
        let claim = claims.get(i).unwrap();
        let current = summary.get(claim.claim_type).unwrap_or(0);
        summary.set(claim.claim_type, current + claim.amount);
    }

    summary
}

/// Get a specific claim by its ID
///
/// # Arguments
/// * `e` - Contract environment
/// * `claim_id` - Unique claim identifier
///
/// # Returns
/// The PendingClaim if found, panics otherwise
///
/// # Panics
/// * If claim_id does not exist
pub fn get_claim_by_id(e: &Env, claim_id: u64) -> PendingClaim {
    e.storage()
        .persistent()
        .get(&DataKey::ClaimById(claim_id))
        .unwrap_or_else(|| panic!("claim not found"))
}

/// Process a single claim by ID (claim-by-ID interface)
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address claiming the reward
/// * `claim_id` - Unique claim identifier to process
///
/// # Returns
/// ClaimResult with details of the processed claim
///
/// # Panics
/// * If claim_id does not exist
/// * If claim is not owned by the user
/// * If claim has already been processed
/// * If claim has expired
/// * If token transfer fails
pub fn process_claim_by_id(e: &Env, user: &Address, claim_id: u64) -> ClaimResult {
    user.require_auth();

    let now = e.ledger().timestamp();
    let mut claim = get_claim_by_id(e, claim_id);

    // Check if claim has already been processed
    if claim.processed {
        panic!("claim already processed");
    }

    // Check if claim has expired
    if claim.expires_at > 0 && now > claim.expires_at {
        panic!("claim has expired");
    }

    // Verify the claim belongs to the user by checking their pending claims
    let user_claims = get_pending_claims(e, user);
    let mut claim_found = false;
    for i in 0..user_claims.len() {
        let user_claim = user_claims.get(i).unwrap();
        if user_claim.claim_id == claim_id {
            claim_found = true;
            break;
        }
    }

    if !claim_found {
        panic!("claim not owned by user");
    }

    // Process the claim
    let total_amount = claim.amount;

    // Mark claim as processed
    claim.processed = true;
    e.storage()
        .persistent()
        .set(&DataKey::ClaimById(claim_id), &claim);

    // Remove from user's pending claims
    let mut remaining_claims = Vec::new(e);
    for i in 0..user_claims.len() {
        let user_claim = user_claims.get(i).unwrap();
        if user_claim.claim_id != claim_id {
            remaining_claims.push_back(user_claim);
        }
    }

    // Update user's claims storage
    if remaining_claims.is_empty() {
        e.storage()
            .persistent()
            .remove(&DataKey::PendingClaims(user.clone()));
        e.storage()
            .persistent()
            .remove(&DataKey::ClaimableAmount(user.clone()));
    } else {
        e.storage()
            .persistent()
            .set(&DataKey::PendingClaims(user.clone()), &remaining_claims);

        let remaining_amount = get_claimable_amount(e, user)
            .checked_sub(total_amount)
            .expect("claimable amount underflow");

        e.storage()
            .persistent()
            .set(&DataKey::ClaimableAmount(user.clone()), &remaining_amount);
    }

    // Transfer tokens to user
    if total_amount > 0 {
        let token: Address = e
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("token not configured");

        let contract = e.current_contract_address();
        soroban_sdk::token::TokenClient::new(e, &token).transfer(&contract, user, &total_amount);
    }

    let mut claim_types = Vec::new(e);
    claim_types.push_back(claim.claim_type);

    let result = ClaimResult {
        processed_count: 1,
        total_amount,
        claim_types,
    };

    // Emit events
    let mut processed_claims = Vec::new(e);
    processed_claims.push_back(claim);
    events::emit_claims_processed(e, user, &result, &processed_claims);

    result
}

/// Get paginated pending claims for a user
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address to check claims for
/// * `offset` - Number of claims to skip (for pagination)
/// * `limit` - Maximum number of claims to return (capped at MAX_BATCH_CLAIMS)
///
/// # Returns
/// Vector of pending claims for the specified page
pub fn get_pending_claims_paginated(
    e: &Env,
    user: &Address,
    offset: u32,
    limit: u32,
) -> Vec<PendingClaim> {
    let all_claims = get_pending_claims(e, user);
    let actual_limit = limit.min(MAX_BATCH_CLAIMS);

    let mut result = Vec::new(e);
    let start = offset.min(all_claims.len() as u32);
    let end = (start + actual_limit).min(all_claims.len() as u32);

    for i in start..end {
        result.push_back(all_claims.get((i as u32).try_into().unwrap()).unwrap());
    }

    result
}

/// Get the count of pending claims for a user
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address to check claims for
///
/// # Returns
/// Number of pending claims
pub fn get_pending_claims_count(e: &Env, user: &Address) -> u32 {
    get_pending_claims(e, user).len()
}

/// Process claims with explicit pagination control
///
/// # Arguments
/// * `e` - Contract environment
/// * `user` - Address claiming rewards
/// * `offset` - Starting index for pagination
/// * `limit` - Maximum number of claims to process (capped at MAX_BATCH_CLAIMS)
/// * `claim_types` - Optional filter for specific claim types (empty = all types)
///
/// # Returns
/// ClaimResult with details of processed claims and next offset
pub fn process_claims_paginated(
    e: &Env,
    user: &Address,
    offset: u32,
    limit: u32,
    claim_types: Vec<ClaimType>,
) -> ClaimResult {
    user.require_auth();

    let now = e.ledger().timestamp();
    let all_claims = get_pending_claims(e, user);

    if all_claims.is_empty() || offset >= all_claims.len() as u32 {
        panic!("no claims to process at offset");
    }

    // Filter claims by type if specified
    let filter_types = !claim_types.is_empty();
    let type_set: Map<ClaimType, bool> = if filter_types {
        let mut map = Map::new(e);
        for i in 0..claim_types.len() {
            map.set(claim_types.get(i).unwrap(), true);
        }
        map
    } else {
        Map::new(e)
    };

    let mut processed_claims = Vec::new(e);
    let mut total_amount = 0i128;
    let mut processed_types = Vec::new(e);
    let actual_limit = limit.min(MAX_BATCH_CLAIMS);
    let mut processed_count = 0u32;

    // Process claims starting from offset
    for i in offset..all_claims.len() {
        if processed_count >= actual_limit {
            break;
        }

        let claim = all_claims.get(i.try_into().unwrap()).unwrap();

        // Skip expired claims
        if claim.expires_at > 0 && now > claim.expires_at {
            continue;
        }

        // Skip if not in filter
        if filter_types && !type_set.contains_key(claim.claim_type) {
            continue;
        }

        // Process this claim
        processed_claims.push_back(claim.clone());
        total_amount = total_amount
            .checked_add(claim.amount)
            .expect("claim total overflow");

        // Track unique claim types
        let mut type_exists = false;
        for j in 0..processed_types.len() {
            if processed_types.get(j).unwrap() == claim.claim_type {
                type_exists = true;
                break;
            }
        }
        if !type_exists {
            processed_types.push_back(claim.claim_type);
        }

        processed_count += 1;
    }

    if processed_claims.is_empty() {
        panic!("no valid claims to process");
    }

    // Remove processed claims from storage
    let mut remaining_claims = Vec::new(e);
    for i in 0..all_claims.len() {
        let claim = all_claims.get(i).unwrap();
        let mut is_processed = false;
        for j in 0..processed_claims.len() {
            if processed_claims.get(j).unwrap().claim_id == claim.claim_id {
                is_processed = true;
                break;
            }
        }
        if !is_processed {
            remaining_claims.push_back(claim);
        }
    }

    // Update storage with remaining claims
    if remaining_claims.is_empty() {
        e.storage()
            .persistent()
            .remove(&DataKey::PendingClaims(user.clone()));
        e.storage()
            .persistent()
            .remove(&DataKey::ClaimableAmount(user.clone()));
    } else {
        e.storage()
            .persistent()
            .set(&DataKey::PendingClaims(user.clone()), &remaining_claims);

        let remaining_amount = get_claimable_amount(e, user)
            .checked_sub(total_amount)
            .expect("claimable amount underflow");

        e.storage()
            .persistent()
            .set(&DataKey::ClaimableAmount(user.clone()), &remaining_amount);
    }

    // Transfer tokens to user
    if total_amount > 0 {
        let token: Address = e
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("token not configured");

        let contract = e.current_contract_address();
        soroban_sdk::token::TokenClient::new(e, &token).transfer(&contract, user, &total_amount);
    }

    let result = ClaimResult {
        processed_count: processed_claims.len(),
        total_amount,
        claim_types: processed_types,
    };

    // Emit events
    events::emit_claims_processed(e, user, &result, &processed_claims);

    result
}
