use crate::*;
use soroban_sdk::{Address, Env, String};

#[cfg(test)]
mod immutable_config_tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup_contract(env: &Env) -> (CredenceBondClient<'_>, Address) {
        let contract_address = env.register(CredenceBond, ());
        let client = CredenceBondClient::new(env, &contract_address);
        let admin = Address::generate(env);
        
        env.mock_all_auths();
        // We don't initialize here because some tests need to do it themselves
        (client, admin)
    }

    #[test]
    #[should_panic]
    fn test_admin_requires_initialization() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);

        // This should panic because not initialized
        client.unregister_attester(&admin); 
    }

    #[test]
    fn test_admin_initialized_correctly() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);

        client.initialize(&admin);

        // This should not panic
        assert_eq!(client.get_admin(), admin);
    }

    #[test]
    fn test_token_requires_initialization() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        client.initialize(&admin);

        assert!(client.get_bond_token().is_none());
    }

    #[test]
    fn test_admin_initialization_is_idempotent() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);

        client.initialize(&admin);
        
        // Second call should not panic (idempotent)
        client.initialize(&admin);
        
        assert_eq!(client.get_admin(), admin);
    }

    #[test]
    fn test_token_can_be_updated() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let token1 = Address::generate(&env);
        let token2 = Address::generate(&env);

        client.initialize(&admin);

        // Set token first time
        client.set_token(&admin, &token1);
        assert_eq!(client.get_bond_token(), Some(token1));

        // Set token second time (overwrite)
        client.set_token(&admin, &token2);
        assert_eq!(client.get_bond_token(), Some(token2));
    }
}
