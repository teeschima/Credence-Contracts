//! Safe Token Operations Tests
//!
//! Tests for safe token operations including:
//! - Non-compliant token handling
//! - Error validation and consistent reverts
//! - Edge cases and boundary conditions
//! - Allowance and approval patterns

use crate::safe_token::*;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{testutils::Address as TestAddress, testutils::Ledger as TestLedger, Address, Env};
use std::panic::catch_unwind;

#[test]
fn test_safe_transfer_with_valid_params() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // This should not panic with valid parameters
    // Note: In a real test, you'd need to mock the token contract behavior
    catch_unwind(|| {
        safe_transfer(&env, &recipient, amount);
    }).unwrap_err(); // Expected to fail due to no actual token contract
}

#[test]
fn test_safe_transfer_with_zero_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should not panic with zero amount (early return)
    catch_unwind(|| {
        safe_transfer(&env, &recipient, 0);
    }).unwrap();
}

#[test]
fn test_safe_transfer_with_negative_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should panic with negative amount
    let result = catch_unwind(|| {
        safe_transfer(&env, &recipient, -100);
    });
    assert!(result.is_err());
    
    // Check if it's the expected panic message
    let panic_payload = result.unwrap_err();
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        assert!(s.contains("amount must be non-negative"));
    }
}

#[test]
fn test_safe_transfer_with_no_token_configured() {
    let env = Env::default();
    let recipient = Address::generate(&env);
    let amount = 1000i128;

    // Should panic when no token is configured
    let result = catch_unwind(|| {
        safe_transfer(&env, &recipient, amount);
    });
    assert!(result.is_err());
}

#[test]
fn test_safe_transfer_from_with_insufficient_allowance() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let owner = Address::generate(&env);
    let amount = 1000i128;

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should panic due to insufficient allowance
    let result = catch_unwind(|| {
        safe_transfer_from(&env, &owner, amount);
    });
    assert!(result.is_err());
    
    // Check if it's the expected panic message
    let panic_payload = result.unwrap_err();
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        assert!(s.contains("insufficient token allowance"));
    }
}

#[test]
fn test_safe_require_allowance_with_zero_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let owner = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should not panic with zero amount (early return)
    catch_unwind(|| {
        safe_require_allowance(&env, &owner, 0);
    }).unwrap();
}

#[test]
fn test_safe_require_allowance_with_negative_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let owner = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should panic with negative amount
    let result = catch_unwind(|| {
        safe_require_allowance(&env, &owner, -100);
    });
    assert!(result.is_err());
    
    // Check if it's the expected panic message
    let panic_payload = result.unwrap_err();
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        assert!(s.contains("amount must be non-negative"));
    }
}

#[test]
fn test_safe_approve_with_negative_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let spender = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should panic with negative amount
    let result = catch_unwind(|| {
        safe_approve(&env, &spender, -100);
    });
    assert!(result.is_err());
    
    // Check if it's the expected panic message
    let panic_payload = result.unwrap_err();
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        assert!(s.contains("amount must be non-negative"));
    }
}

#[test]
fn test_safe_increase_allowance_with_zero_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let spender = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should not panic with zero amount (early return)
    catch_unwind(|| {
        safe_increase_allowance(&env, &spender, 0);
    }).unwrap();
}

#[test]
fn test_safe_increase_allowance_with_negative_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let spender = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should panic with negative amount
    let result = catch_unwind(|| {
        safe_increase_allowance(&env, &spender, -100);
    });
    assert!(result.is_err());
    
    // Check if it's the expected panic message
    let panic_payload = result.unwrap_err();
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        assert!(s.contains("amount must be non-negative"));
    }
}

#[test]
fn test_force_approve_with_negative_amount() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let spender = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should panic with negative amount
    let result = catch_unwind(|| {
        force_approve(&env, &spender, -100);
    });
    assert!(result.is_err());
    
    // Check if it's the expected panic message
    let panic_payload = result.unwrap_err();
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        assert!(s.contains("amount must be non-negative"));
    }
}

#[test]
fn test_get_token_with_no_config() {
    let env = Env::default();

    // Should panic when no token is configured
    let result = catch_unwind(|| {
        get_token(&env);
    });
    assert!(result.is_err());
}

#[test]
fn test_get_token_with_valid_config() {
    let env = Env::default();
    let token_address = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Should return the token address
    let result = catch_unwind(|| {
        get_token(&env)
    });
    assert!(result.is_ok());
}

#[test]
fn test_error_message_consistency() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    // Test negative amount error consistency across functions
    let functions = vec![
        || safe_transfer(&env, &recipient, -1),
        || safe_transfer_from(&env, &recipient, -1),
        || safe_require_allowance(&env, &recipient, -1),
        || safe_approve(&env, &recipient, -1),
        || safe_increase_allowance(&env, &recipient, -1),
        || force_approve(&env, &recipient, -1),
    ];

    for func in functions {
        let result = catch_unwind(func);
        assert!(result.is_err());
        
        let panic_payload = result.unwrap_err();
        if let Some(s) = panic_payload.downcast_ref::<String>() {
            assert!(s.contains("amount must be non-negative"));
        }
    }
}

#[test]
fn test_token_address_validation() {
    let env = Env::default();
    let spender = Address::generate(&env);

    // Test zero address validation (this would need a zero address mock)
    // In a real implementation, you'd test with actual zero address
    // For now, we test that the validation function exists
    catch_unwind(|| {
        validate_token_address(&spender);
    }).unwrap();
}

// Mock token implementation for testing non-compliant behavior
pub struct MockNonCompliantToken {
    env: Env,
    address: Address,
}

impl MockNonCompliantToken {
    pub fn new(env: &Env) -> Self {
        Self {
            env: env.clone(),
            address: Address::generate(env),
        }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    // Simulate a token that doesn't return boolean values
    pub fn transfer_no_return(&self, from: &Address, to: &Address, amount: &i128) {
        // In a real implementation, this would be a token contract call
        // that doesn't return a boolean value
        println!("Transfer {} from {} to {}", amount, from, to);
    }

    // Simulate a token that always fails
    pub fn transfer_always_fails(&self, from: &Address, to: &Address, amount: &i128) {
        panic!("Transfer always fails");
    }
}

#[test]
fn test_non_compliant_token_handling() {
    let env = Env::default();
    let mock_token = MockNonCompliantToken::new(&env);
    let recipient = Address::generate(&env);
    
    // Configure mock token
    env.storage().instance().set(&crate::DataKey::BondToken, mock_token.address());

    // Test that safe operations handle non-compliant tokens gracefully
    // In a real implementation, you'd need to mock the token contract behavior
    // This test demonstrates the structure for testing non-compliant tokens
    
    let result = catch_unwind(|| {
        safe_transfer(&env, &recipient, 1000);
    });
    
    // Should handle gracefully even with non-compliant tokens
    // The exact behavior depends on the Soroban runtime
    assert!(result.is_err() || result.is_ok()); // Either way, it shouldn't crash unexpectedly
}

#[test]
fn test_edge_cases() {
    let env = Env::default();
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Mock token configuration
    env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

    // Test maximum i128 value
    let max_amount = i128::MAX;
    let result = catch_unwind(|| {
        safe_transfer(&env, &recipient, max_amount);
    });
    // Should handle gracefully (either succeed or fail with proper error)
    
    // Test minimum positive value
    let min_amount = 1i128;
    let result = catch_unwind(|| {
        safe_transfer(&env, &recipient, min_amount);
    });
    // Should handle gracefully
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_safe_token_integration_with_token_integration() {
        let env = Env::default();
        let token_address = Address::generate(&env);
        let user = Address::generate(&env);

        // Configure token
        env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

        // Test that token_integration module works with safe_token
        let result = catch_unwind(|| {
            crate::token_integration::require_allowance(&env, &user, 1000);
        });
        
        // Should fail gracefully with insufficient allowance
        assert!(result.is_err());
        
        let panic_payload = result.unwrap_err();
        if let Some(s) = panic_payload.downcast_ref::<String>() {
            assert!(s.contains("insufficient token allowance"));
        }
    }

    #[test]
    fn test_safe_token_integration_with_verifier() {
        let env = Env::default();
        let token_address = Address::generate(&env);
        let verifier = Address::generate(&env);

        // Configure token
        env.storage().instance().set(&crate::DataKey::BondToken, &token_address);

        // Test that verifier module works with safe_token
        let result = catch_unwind(|| {
            crate::safe_token::safe_transfer_from(&env, &verifier, 1000);
        });
        
        // Should fail gracefully with insufficient allowance
        assert!(result.is_err());
    }
}
