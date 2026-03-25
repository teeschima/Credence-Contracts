// ─────────────────────────────────────────────────────────────────────────────
// lib.rs CHANGES — two additions required
// ─────────────────────────────────────────────────────────────────────────────
//
// CHANGE 1: Add GraceWindow variant to the DataKey enum.
//
// Find this in DataKey:
//
//     BondToken,
// }
//
// Replace with:
//
//     BondToken,
//
//     /// Optional grace window in seconds appended to signed-order deadlines.
//     ///
//     /// POLICY NOTE: A grace window absorbs ledger-inclusion delays near the
//     /// expiry boundary, preventing valid signed orders from being rejected due
//     /// to minor timing jitter between signing and on-chain inclusion.
//     ///
//     /// Trade-offs:
//     ///   PRO  — tolerates block-inclusion latency at the deadline boundary.
//     ///   CON  — widens the window a leaked/stolen signature can be submitted.
//     ///          Keep the value small (≤ 60 s). Leave absent (0) in
//     ///          risk-sensitive deployments — grace is DISABLED by default.
//     ///
//     /// Replay protection is NOT weakened: nonces are consumed on first use
//     /// regardless of whether the deadline is nominal or grace-extended.
//     GraceWindow,
// }
//
// ─────────────────────────────────────────────────────────────────────────────
//
// CHANGE 2: Add set_grace_window and get_grace_window public functions.
//
// Find this in the impl block:
//
//     pub fn get_bond_token(e: Env) -> Option<Address> {
//         e.storage().instance().get(&DataKey::BondToken)
//     }
//
// Add directly AFTER it:

    pub fn set_grace_window(e: Env, admin: Address, seconds: u64) {
        admin.require_auth();
        Self::require_admin_internal(&e, &admin);
        if seconds == 0 {
            e.storage().instance().remove(&DataKey::GraceWindow);
        } else {
            e.storage().instance().set(&DataKey::GraceWindow, &seconds);
        }
    }

    pub fn get_grace_window(e: Env) -> u64 {
        e.storage()
            .instance()
            .get(&DataKey::GraceWindow)
            .unwrap_or(0)
    }

// ─────────────────────────────────────────────────────────────────────────────
//
// CHANGE 3: Register the new test module.
//
// In the #[cfg(test)] block at the bottom of lib.rs, add:
//
//     #[cfg(test)]
//     mod test_grace_period;
//
// Place it alphabetically between test_fees and test_governance_approval.
//
// ─────────────────────────────────────────────────────────────────────────────