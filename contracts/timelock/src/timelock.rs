//! # Credence Timelock Contract
//!
//! Enforces a mandatory delay before protocol parameter changes take effect.
//! Changes must be proposed by the admin, wait for a minimum delay period,
//! and can be cancelled by governance during the waiting period.

use credence_errors::ContractError;
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, Address, Env, Symbol};

/// Execution grace period in seconds after ETA.
pub const EXECUTION_GRACE_PERIOD: u64 = 86_400;

/// A pending parameter change. Created when admin proposes a new value for a
/// protocol parameter. The change can only be executed after the ETA (estimated
/// time of arrival) has passed.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ParameterChange {
    /// Identifier for the parameter being changed.
    pub parameter_key: Symbol,
    /// The proposed new value.
    pub new_value: i128,
    /// Ledger timestamp when the change was proposed.
    pub proposed_at: u64,
    /// Earliest timestamp at which the change can be executed.
    pub eta: u64,
    /// Latest timestamp at which the change can be executed.
    pub expires_at: u64,
    /// Minimum delay enforced when the change was queued.
    pub min_delay_at_queue: u64,
    /// True once the change has been executed.
    pub executed: bool,
    /// True if the change was cancelled by governance.
    pub cancelled: bool,
}

#[contracttype]
pub enum DataKey {
    /// Contract administrator who can propose and execute changes.
    Admin,
    /// Governance address that can cancel pending changes.
    GovernanceAddress,
    /// Minimum delay in seconds between proposal and execution.
    MinDelay,
    /// A pending or completed parameter change, indexed by ID.
    PendingChange(u64),
    /// Counter for generating unique change IDs.
    ChangeCounter,
    Paused,
    PauseSigner(Address),
    PauseSignerCount,
    PauseThreshold,
    PauseProposalCounter,
    PauseProposal(u64),
    PauseApproval(u64, Address),
    PauseApprovalCount(u64),
}

#[contract]
pub struct Timelock;

#[contractimpl]
impl Timelock {
    /// Initialize the timelock contract.
    ///
    /// @param e          The contract environment
    /// @param admin      Address that can propose and execute parameter changes
    /// @param governance Address that can cancel pending changes
    /// @param min_delay  Minimum delay in seconds before a change can be executed
    pub fn initialize(e: Env, admin: Address, governance: Address, min_delay: u64) {
        if e.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&e, ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        if min_delay == 0 {
            panic_with_error!(&e, ContractError::AmountMustBePositive);
        }
        e.storage().instance().set(&DataKey::Admin, &admin);
        e.storage().instance().set(&DataKey::Paused, &false);
        e.storage()
            .instance()
            .set(&DataKey::PauseSignerCount, &0_u32);
        e.storage().instance().set(&DataKey::PauseThreshold, &0_u32);
        e.storage()
            .instance()
            .set(&DataKey::PauseProposalCounter, &0_u64);
        e.storage()
            .instance()
            .set(&DataKey::GovernanceAddress, &governance);
        e.storage().instance().set(&DataKey::MinDelay, &min_delay);
        e.storage().instance().set(&DataKey::ChangeCounter, &0_u64);
        e.events().publish(
            (Symbol::new(&e, "timelock_initialized"),),
            (admin, governance, min_delay),
        );
    }

    /// Propose a parameter change. Only the admin can propose. The change is
    /// queued with an ETA of `now + min_delay`.
    ///
    /// @param e             The contract environment
    /// @param proposer      Must be the admin
    /// @param parameter_key Identifier for the parameter to change
    /// @param new_value     The proposed new value
    /// @return change_id    Unique ID for this pending change
    pub fn propose_change(
        e: Env,
        proposer: Address,
        parameter_key: Symbol,
        new_value: i128,
    ) -> u64 {
        crate::pausable::require_not_paused(&e);
        let min_delay: u64 = e
            .storage()
            .instance()
            .get(&DataKey::MinDelay)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized));

        if min_delay == 0 {
            panic_with_error!(&e, ContractError::AmountMustBePositive);
        }

        let eta = e
            .ledger()
            .timestamp()
            .checked_add(min_delay)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::Overflow));

        Self::queue_change(e, proposer, parameter_key, new_value, eta)
    }

    /// Queue a parameter change with an explicit ETA. Only the admin can queue.
    ///
    /// @param e             The contract environment
    /// @param proposer      Must be the admin
    /// @param parameter_key Identifier for the parameter to change
    /// @param new_value     The proposed new value
    /// @param eta           Earliest timestamp for execution; must satisfy min delay
    /// @return change_id    Unique ID for this pending change
    pub fn queue_change(
        e: Env,
        proposer: Address,
        parameter_key: Symbol,
        new_value: i128,
        eta: u64,
    ) -> u64 {
        crate::pausable::require_not_paused(&e);
        proposer.require_auth();
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized));
        if proposer != admin {
            panic_with_error!(&e, ContractError::NotAdmin);
        }

        let min_delay: u64 = e
            .storage()
            .instance()
            .get(&DataKey::MinDelay)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized));

        if min_delay == 0 {
            panic_with_error!(&e, ContractError::AmountMustBePositive);
        }

        let now = e.ledger().timestamp();
        let earliest_eta = now
            .checked_add(min_delay)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::Overflow));
        if eta < earliest_eta {
            panic_with_error!(&e, ContractError::InvalidPauseAction);
        }

        let expires_at = eta
            .checked_add(EXECUTION_GRACE_PERIOD)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::Overflow));

        let id: u64 = e
            .storage()
            .instance()
            .get(&DataKey::ChangeCounter)
            .unwrap_or(0);
        let next_id = id
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::Overflow));
        e.storage()
            .instance()
            .set(&DataKey::ChangeCounter, &next_id);

        let change = ParameterChange {
            parameter_key: parameter_key.clone(),
            new_value,
            proposed_at: now,
            eta,
            expires_at,
            min_delay_at_queue: min_delay,
            executed: false,
            cancelled: false,
        };

        e.storage()
            .persistent()
            .set(&DataKey::PendingChange(id), &change);

        e.events().publish(
            (Symbol::new(&e, "change_proposed"), id),
            (parameter_key, new_value, eta),
        );
        id
    }

    /// Execute a pending parameter change. Only the admin can execute.
    /// The current ledger timestamp must be at or past the change's ETA.
    ///
    /// @param e         The contract environment
    /// @param change_id ID of the change to execute
    pub fn execute_change(e: Env, change_id: u64) {
        crate::pausable::require_not_paused(&e);
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized));
        admin.require_auth();

        let mut change: ParameterChange = e
            .storage()
            .persistent()
            .get(&DataKey::PendingChange(change_id))
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::ProposalNotFound));

        if change.cancelled {
            panic_with_error!(&e, ContractError::AlreadyRevoked);
        }
        if change.executed {
            panic_with_error!(&e, ContractError::ProposalAlreadyExecuted);
        }

        let now = e.ledger().timestamp();
        if now < change.eta {
            // Early execution forbidden: must be at or after ETA.
            panic_with_error!(&e, ContractError::InvalidPauseAction);
        }
        if now > change.expires_at {
            // Late execution forbidden: must be at or before expires_at.
            panic_with_error!(&e, ContractError::InvalidPauseAction);
        }

        if change.min_delay_at_queue == 0 {
            panic_with_error!(&e, ContractError::AmountMustBePositive);
        }

        let earliest_eta = change
            .proposed_at
            .checked_add(change.min_delay_at_queue)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::Overflow));
        if change.eta < earliest_eta {
            panic_with_error!(&e, ContractError::InvalidPauseAction);
        }

        change.executed = true;
        e.storage()
            .persistent()
            .set(&DataKey::PendingChange(change_id), &change);

        e.events().publish(
            (Symbol::new(&e, "change_executed"), change_id),
            (change.parameter_key.clone(), change.new_value),
        );
    }

    /// Cancel a pending parameter change. Only the governance address can cancel.
    ///
    /// @param e         The contract environment
    /// @param canceller Must be the governance address
    /// @param change_id ID of the change to cancel
    pub fn cancel_change(e: Env, canceller: Address, change_id: u64) {
        crate::pausable::require_not_paused(&e);
        canceller.require_auth();
        let governance: Address = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceAddress)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized));
        if canceller != governance {
            panic_with_error!(&e, ContractError::NotAdmin);
        }

        let mut change: ParameterChange = e
            .storage()
            .persistent()
            .get(&DataKey::PendingChange(change_id))
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::ProposalNotFound));

        if change.executed {
            panic_with_error!(&e, ContractError::ProposalAlreadyExecuted);
        }
        if change.cancelled {
            panic_with_error!(&e, ContractError::AlreadyRevoked);
        }

        change.cancelled = true;
        e.storage()
            .persistent()
            .set(&DataKey::PendingChange(change_id), &change);

        e.events().publish(
            (Symbol::new(&e, "change_cancelled"), change_id),
            change.parameter_key.clone(),
        );
    }

    /// Update the minimum delay. Only the admin can call this.
    /// The new delay must be greater than zero.
    ///
    /// @param e         The contract environment
    /// @param new_delay New minimum delay in seconds
    pub fn update_min_delay(e: Env, new_delay: u64) {
        crate::pausable::require_not_paused(&e);
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized));
        admin.require_auth();

        if new_delay == 0 {
            panic_with_error!(&e, ContractError::AmountMustBePositive);
        }

        let old_delay: u64 = e.storage().instance().get(&DataKey::MinDelay).unwrap_or(0);

        e.storage().instance().set(&DataKey::MinDelay, &new_delay);

        e.events()
            .publish((Symbol::new(&e, "delay_updated"),), (old_delay, new_delay));
    }

    /// Get a parameter change by ID.
    pub fn get_change(e: Env, change_id: u64) -> ParameterChange {
        e.storage()
            .persistent()
            .get(&DataKey::PendingChange(change_id))
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::ProposalNotFound))
    }

    /// Get the current minimum delay.
    pub fn get_min_delay(e: Env) -> u64 {
        e.storage()
            .instance()
            .get(&DataKey::MinDelay)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized))
    }

    /// Get the admin address.
    pub fn get_admin(e: Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized))
    }

    /// Get the governance address.
    pub fn get_governance(e: Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::GovernanceAddress)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotInitialized))
    }

    pub fn pause(e: Env, caller: Address) -> Option<u64> {
        crate::pausable::pause(&e, &caller)
    }

    pub fn unpause(e: Env, caller: Address) -> Option<u64> {
        crate::pausable::unpause(&e, &caller)
    }

    pub fn is_paused(e: Env) -> bool {
        crate::pausable::is_paused(&e)
    }

    pub fn set_pause_signer(e: Env, admin: Address, signer: Address, enabled: bool) {
        crate::pausable::set_pause_signer(&e, &admin, &signer, enabled)
    }

    pub fn set_pause_threshold(e: Env, admin: Address, threshold: u32) {
        crate::pausable::set_pause_threshold(&e, &admin, threshold)
    }

    pub fn approve_pause_proposal(e: Env, signer: Address, proposal_id: u64) {
        crate::pausable::approve_pause_proposal(&e, &signer, proposal_id)
    }

    pub fn execute_pause_proposal(e: Env, proposal_id: u64) {
        crate::pausable::execute_pause_proposal(&e, proposal_id)
    }
}
