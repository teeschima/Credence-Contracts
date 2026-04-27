use crate::*;
use soroban_sdk::{Address, Env, String};
use soroban_sdk::testutils::{Address as _};

#[cfg(test)]
mod zero_address_tests {
    use super::*;

    fn setup_contract(env: &Env) -> (CredenceBondClient<'_>, Address) {
        let contract_address = env.register(CredenceBond, ());
        let client = CredenceBondClient::new(env, &contract_address);
        let admin = Address::generate(env);
        
        env.mock_all_auths();
        client.initialize(&admin);

        (client, admin)
    }

    fn get_zero_address(env: &Env) -> Address {
        Address::from_string(&String::from_str(
            env,
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ))
    }

    #[test]
    #[should_panic]
    fn test_set_early_exit_config_rejects_zero_address() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);

        client.set_early_exit_config(&admin, &zero_address, &100_u32);
    }

    #[test]
    #[should_panic]
    fn test_set_emergency_config_rejects_zero_governance() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);
        let valid_address = Address::generate(&env);

        client.set_emergency_config(&admin, &zero_address, &valid_address, &50_u32, &true);
    }

    #[test]
    #[should_panic]
    fn test_set_emergency_config_rejects_zero_treasury() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);
        let valid_address = Address::generate(&env);

        client.set_emergency_config(&admin, &valid_address, &zero_address, &50_u32, &true);
    }

    #[test]
    #[should_panic]
    fn test_register_attester_rejects_zero_address() {
        let env = Env::default();
        let (client, _admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);

        client.register_attester(&zero_address);
    }

    #[test]
    #[should_panic]
    fn test_register_verifier_rejects_zero_address() {
        let env = Env::default();
        let (client, _admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);

        client.register_verifier(&zero_address, &1000_i128);
    }

    #[test]
    #[should_panic]
    fn test_set_token_rejects_zero_address() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);

        client.set_token(&admin, &zero_address);
    }

    #[test]
    #[should_panic]
    fn test_set_usdc_token_rejects_zero_address() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let zero_address = get_zero_address(&env);
        let network = String::from_str(&env, "mainnet");

        client.set_usdc_token(&admin, &zero_address, &network);
    }

    #[test]
    fn test_valid_addresses_succeed() {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let treasury = Address::generate(&env);
        let governance = Address::generate(&env);
        let attester = Address::generate(&env);
        let verifier = Address::generate(&env);
        let token = Address::generate(&env);
        let network = String::from_str(&env, "mainnet");

        // These should all succeed
        client.set_early_exit_config(&admin, &treasury, &100_u32);

        client.set_emergency_config(&admin, &governance, &treasury, &50_u32, &true);

        client.register_attester(&attester);

        client.register_verifier(&verifier, &1000_i128);

        client.set_token(&admin, &token);

        client.set_usdc_token(&admin, &token, &network);
    }
}
