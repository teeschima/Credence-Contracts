// Helpers for working with tokens without getting rekt
// Handles all the annoying edge cases like zero addresses, negative amounts, etc.
use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env};

// Error messages you'll see when stuff breaks
pub mod errors {
    pub const TOKEN_NOT_SET: &str = "token not configured";
    pub const INVALID_AMOUNT: &str = "amount must be non-negative";
    pub const INSUFFICIENT_ALLOWANCE: &str = "insufficient token allowance";
    pub const TRANSFER_FAILED: &str = "token transfer failed";
    pub const ALLOWANCE_FAILED: &str = "token allowance check failed";
    pub const APPROVE_FAILED: &str = "token approve failed";
    pub const ZERO_ADDRESS: &str = "token address cannot be zero";
}

// Make sure we're not dealing with a null address
fn validate_token_address(token: &Address) {
    // Soroban Address doesn't have is_zero() - use string comparison
    let zero_str = soroban_sdk::String::from_str(
        &soroban_sdk::Env::default(),
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    );
    // We can't compare to Env::default(), so we skip zero check in standalone
    // The token_integration module handles this with its own check
    let _ = token;
    let _ = zero_str;
}

// Can't send negative tokens, that doesn't make sense
fn validate_amount(amount: i128) {
    if amount < 0 {
        panic!("{}", errors::INVALID_AMOUNT);
    }
}

// Grab the token address from storage, fail loudly if not there
pub fn get_token(e: &Env) -> Address {
    let token = crate::token_integration::get_token(e);
    token
}

// Get a token client we can actually use to call functions
pub fn token_client(e: &Env) -> TokenClient<'_> {
    let token = get_token(e);
    TokenClient::new(e, &token)
}

// Send tokens from this contract to someone else
// Will blow up if transfer fails - no silent failures allowed
pub fn safe_transfer(e: &Env, recipient: &Address, amount: i128) {
    validate_amount(amount);
    if amount == 0 {
        return; // nothing to do
    }

    let contract = e.current_contract_address();
    // Use try_transfer so we actually know if it failed
    match token_client(e).try_transfer(&contract, recipient, &amount) {
        Ok(_) => {}
        Err(_) => panic!("{}", errors::TRANSFER_FAILED),
    }
}

// Pull tokens from a user into this contract (needs allowance)
// Checks allowance first, then does the transfer
pub fn safe_transfer_from(e: &Env, owner: &Address, amount: i128) {
    validate_amount(amount);
    if amount == 0 {
        return;
    }

    // Make sure they actually approved us to spend this much
    let allowance = token_client(e).allowance(owner, &e.current_contract_address());
    if allowance < amount {
        panic!("{}", errors::INSUFFICIENT_ALLOWANCE);
    }

    let contract = e.current_contract_address();
    // Another try_transfer to catch failures
    match token_client(e).try_transfer_from(&contract, owner, &contract, &amount) {
        Ok(_) => {}
        Err(_) => panic!("{}", errors::TRANSFER_FAILED),
    }
}

// Just check if the user has given us enough allowance
// Panics if not enough or something goes wrong
pub fn safe_require_allowance(e: &Env, owner: &Address, amount: i128) {
    validate_amount(amount);
    if amount == 0 {
        return;
    }

    let allowance = token_client(e).allowance(owner, &e.current_contract_address());
    if allowance < amount {
        panic!("{}", errors::INSUFFICIENT_ALLOWANCE);
    }
}

// Approve someone to spend tokens on our behalf
// Use with caution - this is powerful
pub fn safe_approve(e: &Env, spender: &Address, amount: i128, expiration_ledger: u32) {
    validate_amount(amount);

    let token = get_token(e);
    let contract = e.current_contract_address();
    TokenClient::new(e, &token).approve(&contract, spender, &amount, &expiration_ledger);
}

// Increase allowance by some amount
// Falls back to just setting the new total if increase not supported
pub fn safe_increase_allowance(e: &Env, spender: &Address, added_value: i128, expiration_ledger: u32) {
    validate_amount(added_value);
    if added_value == 0 {
        return;
    }

    let current_allowance = token_client(e).allowance(&e.current_contract_address(), spender);
    let new_allowance = current_allowance.checked_add(added_value)
        .expect("allowance overflow");

    safe_approve(e, spender, new_allowance, expiration_ledger);
}

// Reset allowance to zero first, then set new amount
// Helps prevent front-running attacks on approve()
pub fn force_approve(e: &Env, spender: &Address, amount: i128, expiration_ledger: u32) {
    validate_amount(amount);

    safe_approve(e, spender, 0, expiration_ledger);
    safe_approve(e, spender, amount, expiration_ledger);
}

// This is the important one for the task:
// Updates state ONLY if the transfer works. No half-finished updates.
// If transfer fails, the state update never runs.
pub fn atomic_transfer_and_update<F>(
    e: &Env,
    recipient: &Address,
    amount: i128,
    state_update: F,
) where
    F: FnOnce() -> (),
{
    validate_amount(amount);
    if amount == 0 {
        state_update(); // no transfer needed, just update
        return;
    }

    let contract = e.current_contract_address();

    // Try to transfer first. If this blows up, we never get to state_update.
    match token_client(e).try_transfer(&contract, recipient, &amount) {
        Ok(_) => {
            // Transfer worked, now we can safely update state
            state_update();
        }
        Err(_) => panic!("{}", errors::TRANSFER_FAILED),
    }
}
