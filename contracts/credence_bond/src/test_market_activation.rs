//! Market Activation Validation Tests
//!
//! Test suite for comprehensive pre-activation parameter validation.
//! Ensures bonds cannot be activated with incomplete or invalid risk parameters.
//! Addresses issue #179: Validate market listing parameters before activation.

use crate::test_helpers::{create_test_env, setup_contract, create_test_token};
use soroban_sdk::{Address, Env, Error};

#[test]
fn test_valid_bond_activation_succeeds() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    
    // Configure all required parameters
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50); // 0.5% fee
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_silver_threshold(&e, admin.clone(), 1_000_000_000);
    crate::parameters::set_gold_threshold(&e, admin.clone(), 10_000_000_000);
    crate::parameters::set_platinum_threshold(&e, admin.clone(), 100_000_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    crate::verifier::set_min_stake(&e, admin.clone(), 10_000_000);
    crate::cooldown::set_cooldown_period(&e, admin.clone(), 86_400); // 1 day
    
    // Valid bond creation should succeed
    let bond = crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000); // 30 days
    
    assert!(bond.active);
    assert_eq!(bond.bonded_amount, 995_000_000); // After 0.5% fee
    assert_eq!(bond.bond_duration, 2_592_000);
}

#[test]
#[should_panic(expected = "bond token not configured - cannot activate bond")]
fn test_activation_fails_without_token_config() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    
    setup_contract(&e, admin.clone());
    
    // Skip token configuration - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "fee treasury not configured - cannot activate bond")]
fn test_activation_fails_without_fee_config() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    
    // Skip fee configuration - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "bronze threshold not configured - cannot activate bond")]
fn test_activation_fails_with_zero_bronze_threshold() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    
    // Don't set bronze threshold (defaults to 0) - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "silver threshold must be greater than bronze threshold")]
fn test_activation_fails_with_invalid_tier_thresholds() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    
    // Set invalid tier thresholds (silver <= bronze)
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 1_000_000_000);
    crate::parameters::set_silver_threshold(&e, admin.clone(), 1_000_000_000); // Same as bronze
    
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 2_000_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "max leverage not configured - cannot activate bond")]
fn test_activation_fails_without_max_leverage() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    
    // Skip max leverage configuration - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "bond amount below minimum threshold (bronze tier)")]
fn test_activation_fails_with_amount_below_threshold() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 1_000_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    
    // Amount below bronze threshold - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 500_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "bond amount exceeds maximum leverage limit")]
fn test_activation_fails_with_excessive_leverage() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 10); // Very low leverage
    
    // Amount that exceeds leverage limit - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 100_000_000_000, 2_592_000);
}

#[test]
#[should_panic(expected = "rolling bonds require non-zero notice period duration")]
fn test_rolling_bond_fails_with_zero_notice_period() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    crate::cooldown::set_cooldown_period(&e, admin.clone(), 86_400);
    
    // Rolling bond with zero notice period - should fail
    crate::CredenceBond::create_bond_with_rolling(
        e.clone(), 
        user.clone(), 
        1_000_000_000, 
        2_592_000, 
        true, 
        0 // Zero notice period
    );
}

#[test]
#[should_panic(expected = "notice period cannot exceed bond duration")]
fn test_rolling_bond_fails_with_excessive_notice_period() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    crate::cooldown::set_cooldown_period(&e, admin.clone(), 86_400);
    
    // Notice period longer than bond duration - should fail
    crate::CredenceBond::create_bond_with_rolling(
        e.clone(), 
        user.clone(), 
        1_000_000_000, 
        86_400, // 1 day
        true, 
        172_800 // 2 day notice period (longer than bond)
    );
}

#[test]
#[should_panic(expected = "notice period must be at least cooldown period")]
fn test_rolling_bond_fails_with_insufficient_notice_period() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    crate::cooldown::set_cooldown_period(&e, admin.clone(), 604_800); // 7 days
    
    // Notice period shorter than cooldown - should fail
    crate::CredenceBond::create_bond_with_rolling(
        e.clone(), 
        user.clone(), 
        1_000_000_000, 
        2_592_000, // 30 days
        true, 
        86_400 // 1 day notice (shorter than 7 day cooldown)
    );
}

#[test]
#[should_panic(expected = "emergency mode enabled but governance not configured - cannot activate bond")]
fn test_activation_fails_with_incomplete_emergency_config() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    let governance = Address::generate(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    
    // Enable emergency mode but don't set governance - should fail
    crate::CredenceBond::set_emergency_config(
        e.clone(),
        admin.clone(),
        Address::generate(&e), // Different governance
        admin.clone(),
        100, // 1% fee
        true
    );
    
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
}

#[test]
fn test_inactive_bond_cannot_accept_actions() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    
    // Create bond
    let bond = crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
    assert!(bond.active);
    
    // Manually deactivate bond
    let key = crate::DataKey::Bond;
    let mut inactive_bond = bond;
    inactive_bond.active = false;
    e.storage().instance().set(&key, &inactive_bond);
    
    // Verify bond is inactive
    assert!(!crate::CredenceBond::is_bond_active(e.clone()));
    
    // Withdrawal should fail on inactive bond
    let result = std::panic::catch_unwind(|| {
        crate::CredenceBond::withdraw_bond(e.clone(), 100_000_000);
    });
    assert!(result.is_err());
    let panic_msg = result.unwrap_err().downcast::<String>().unwrap();
    assert!(panic_msg.contains("bond not active"));
}

#[test]
fn test_is_bond_active_functionality() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    
    // No bond exists - should return false
    assert!(!crate::CredenceBond::is_bond_active(e.clone()));
    
    // Create active bond - should return true
    let bond = crate::CredenceBond::create_bond(e.clone(), user.clone(), 1_000_000_000, 2_592_000);
    assert!(bond.active);
    assert!(crate::CredenceBond::is_bond_active(e.clone()));
    
    // Deactivate bond - should return false
    let key = crate::DataKey::Bond;
    let mut inactive_bond = bond;
    inactive_bond.active = false;
    e.storage().instance().set(&key, &inactive_bond);
    assert!(!crate::CredenceBond::is_bond_active(e.clone()));
}

#[test]
fn test_valid_rolling_bond_activation() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    crate::cooldown::set_cooldown_period(&e, admin.clone(), 86_400);
    
    // Valid rolling bond should succeed
    let bond = crate::CredenceBond::create_bond_with_rolling(
        e.clone(), 
        user.clone(), 
        1_000_000_000, 
        2_592_000, // 30 days
        true, 
        172_800 // 2 day notice period
    );
    
    assert!(bond.active);
    assert!(bond.is_rolling);
    assert_eq!(bond.notice_period_duration, 172_800);
}

#[test]
#[should_panic(expected = "bond amount cannot be zero")]
fn test_activation_fails_with_zero_amount() {
    let e = create_test_env();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = create_test_token(&e);
    
    setup_contract(&e, admin.clone());
    crate::token_integration::set_token(&e, &admin, &token);
    crate::fees::set_config(&e, admin.clone(), admin.clone(), 50);
    crate::parameters::set_bronze_threshold(&e, admin.clone(), 100_000_000);
    crate::parameters::set_max_leverage(&e, admin.clone(), 100_000);
    
    // Zero amount - should fail
    crate::CredenceBond::create_bond(e.clone(), user.clone(), 0, 2_592_000);
}
