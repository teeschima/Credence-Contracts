# Security

Security mechanisms for the Credence bond and attestation system.

## Overflow-safe arithmetic

- **Checked arithmetic** — All financial calculations use checked arithmetic to prevent overflows/underflows.
- **Failure mode** — Overflows/underflows/div-by-zero revert execution with a descriptive error message.
- **Implementation** — Shared helpers live in `contracts/credence_bond/src/math.rs` and are used for basis-point math and other amount operations.

## Replay attack prevention

- **Nonces** — Each identity has a nonce (starts at 0). State-changing attestation calls require the current nonce and increment it on success.
- **get_nonce(identity)** — Returns the current nonce; the caller must pass this value in the next add_attestation or revoke_attestation call.
- Replayed or out-of-order transactions are rejected with "invalid nonce" because the stored nonce no longer matches.
- Nonce overflow is handled by checked arithmetic (panic if increment would overflow).

## Attestation security

- Only registered attesters can add attestations; attester must pass require_auth.
- Duplicate attestations (same verifier, identity, attestation_data) are rejected.
- Revocation is restricted to the original verifier; nonce is required for revoke.

## Bond and reentrancy

- Reentrancy guard is used in withdraw_bond, slash_bond, and collect_fees; state is updated before any external call (checks-effects-interactions).
- See contract code for lock acquire/release around callbacks.

## Same-ledger sequencing guardrails

- Credence enforces a same-ledger guard for sensitive bond operations to prevent unfair sandwich-style ordering attacks.
- The `same_ledger_liquidation_guard` records the current ledger sequence after every collateral-increasing action, including bond creation, top-up, and batch creation.
- `slash_bond` rejects any slash if the last collateral increase occurred in the current ledger, preventing a slash from executing in the same ledger as an increase.
- This is a targeted anti-sandwich protection and does not block unrelated operations such as attestations, withdrawals, or governance actions.
