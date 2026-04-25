# Security Analysis: Reentrancy Protection

## Overview

This document describes the reentrancy attack vectors relevant to the Credence Bond contract, the protection mechanisms in place, and the test results verifying their effectiveness.

For other security topics (including overflow-safe arithmetic for financial calculations), see `docs/security.md`.

> **Last updated**: security/bond-reentrancy-hardening-fresh — hardened state-update ordering across `withdraw_bond`, `withdraw_early`, and `execute_cooldown_withdrawal`; restricted `set_callback` to admin only; added 8 attacker-harness regression tests.

## Reentrancy in Soroban vs EVM

Unlike EVM-based contracts (Solidity), Soroban smart contracts on Stellar benefit from **runtime-level reentrancy protection**. The Soroban VM prevents a contract from being re-entered while it is already executing — any cross-contract call that attempts to invoke the originating contract will fail with:

```
HostError: Error(Context, InvalidAction)
"Contract re-entry is not allowed"
```

This is a fundamental architectural advantage over EVM, where reentrancy must be handled entirely at the application level.

## Defense-in-Depth: Application-Level Guards

Despite Soroban's built-in protection, the Credence Bond contract implements an **application-level reentrancy guard** as a defense-in-depth measure. This protects against:

- Future changes to the Soroban runtime behavior
- Logical reentrancy through indirect call chains
- State consistency during external interactions

### Guard Implementation

The guard uses a boolean `locked` flag in instance storage:

| Function | Description |
|---|---|
| `acquire_lock()` | Sets `locked = true`; panics with `"reentrancy detected"` if already locked |
| `release_lock()` | Sets `locked = false` |
| `check_lock()` | Returns current lock state |

### Protected Functions

All external-call-bearing functions use the guard:

| Function | Lock status | Callback |
|----------|-------------|---------|
| `withdraw_bond_full()` | ✅ guarded | `on_withdraw` |
| `withdraw_bond()` | ✅ guarded (hardened) | `on_withdraw` |
| `withdraw_early()` | ✅ guarded | `on_withdraw` |
| `execute_cooldown_withdrawal()` | ✅ guarded | `on_withdraw` |
| `slash_bond()` | ✅ guarded | `on_slash` |
| `collect_fees()` | ✅ guarded | `on_collect` |

Each function follows the **checks-effects-interactions** (CEI) pattern:
1. Acquire reentrancy lock
2. Validate inputs and authorization (Checks)
3. Update state (Effects) **before** any external call
4. Invoke callback (Interaction — blocked by held lock if re-entered)
5. Perform token transfer (Interaction — final external call)
6. Release reentrancy lock

### Hardening: CEI Fixes (2026-04)

Three functions previously violated CEI by calling `token_integration::transfer_from_contract`
**before** committing state updates. A malicious token or callback registered as the contract
callback could have exploited this ordering to observe or re-enter the contract in an
intermediate state.

| Function | Before fix | After fix |
|----------|-----------|----------|
| `withdraw_bond()` | Transfer → state update | State update → callback → transfer ✅ |
| `withdraw_early()` | Transfer → state update | State update → callback → transfer ✅ |
| `execute_cooldown_withdrawal()` | State update ✅ | Added `on_withdraw` callback after state ✅ |

`withdraw_bond()` also lacked a reentrancy guard entirely before this fix.

## Attack Vectors Tested

### 1. Same-Function Reentrancy
An attacker contract registered as a callback attempts to re-enter the same function during execution:
- `withdraw_bond` → `on_withdraw` callback → `withdraw_bond` (re-entry)
- `slash_bond` → `on_slash` callback → `slash_bond` (re-entry)
- `collect_fees` → `on_collect` callback → `collect_fees` (re-entry)

**Result**: All blocked by Soroban runtime (`HostError: Error(Context, InvalidAction)`).

### 2. Cross-Function Reentrancy
An attacker contract attempts to call a *different* guarded function during a callback:
- `withdraw_bond` → `on_withdraw` callback → `slash_bond` (cross-function re-entry)

**Result**: Blocked by Soroban runtime. The application-level guard would also catch this since all guarded functions share the same lock.

### 3. State Consistency After Operations
Verified that the reentrancy lock is:
- Not held before any operation
- Released after successful `withdraw_bond`
- Released after successful `slash_bond`
- Released after successful `collect_fees`

### 4. Sequential Operation Safety
Multiple guarded operations called in sequence (slash → collect fees → withdraw) all succeed, confirming the lock is properly released between calls.

## Test Summary

| # | Test | Type | Result |
|---|------|------|--------|
| 1 | `test_withdraw_reentrancy_blocked` | Same-function reentrancy (`withdraw_bond_full`) | PASS (blocked) |
| 2 | `test_slash_reentrancy_blocked` | Same-function reentrancy (`slash_bond`) | PASS (blocked) |
| 3 | `test_fee_collection_reentrancy_blocked` | Same-function reentrancy (`collect_fees`) | PASS (blocked) |
| 4 | `test_lock_not_held_initially` | State lock verification | PASS |
| 5 | `test_lock_released_after_withdraw` | State lock verification | PASS |
| 6 | `test_lock_released_after_slash` | State lock verification | PASS |
| 7 | `test_lock_released_after_fee_collection` | State lock verification | PASS |
| 8 | `test_normal_withdraw_succeeds` | Happy path | PASS |
| 9 | `test_normal_slash_succeeds` | Happy path | PASS |
| 10 | `test_normal_fee_collection_succeeds` | Happy path | PASS |
| 11 | `test_sequential_operations_succeed` | Sequential safety | PASS |
| 12 | `test_slash_exceeds_bond_rejected` | Input validation | PASS |
| 13 | `test_withdraw_non_owner_rejected` | Authorization | PASS |
| 14 | `test_double_withdraw_rejected` | State transition | PASS |
| 15 | `test_cross_function_reentrancy_blocked` | Cross-function reentrancy | PASS |
| 16 | `test_partial_withdraw_reentrancy_blocked` | Same-function reentrancy (`withdraw_bond`) — **new** | PASS (blocked) |
| 17 | `test_withdraw_early_reentrancy_blocked` | Same-function reentrancy (`withdraw_early`) — **new** | PASS (blocked) |
| 18 | `test_cooldown_withdrawal_reentrancy_blocked` | Same-function reentrancy (`execute_cooldown_withdrawal`) — **new** | PASS (blocked) |
| 19 | `test_set_callback_non_admin_rejected` | Admin gate on `set_callback` — **new** | PASS |
| 20 | `test_state_committed_before_callback_withdraw_bond` | CEI ordering (`withdraw_bond`) — **new** | PASS |
| 21 | `test_state_committed_before_callback_slash` | CEI ordering (`slash_bond`) — **new** | PASS |
| 22 | `test_lock_released_after_partial_withdraw` | State lock verification (`withdraw_bond`) — **new** | PASS |

**22 reentrancy-specific regression tests.**

## Malicious Contract Mocks

Five attacker/mock contracts were created for testing:

| Mock | Behavior |
|------|----------|
| `WithdrawAttacker` | Re-enters `withdraw_bond` from `on_withdraw` callback |
| `SlashAttacker` | Re-enters `slash_bond` from `on_slash` callback |
| `FeeAttacker` | Re-enters `collect_fees` from `on_collect` callback |
| `CrossAttacker` | Calls `slash_bond` from `on_withdraw` callback (cross-function) |
| `BenignCallback` | No-op callbacks for happy-path testing with external calls |

## Key Finding

**Soroban provides runtime-level reentrancy protection.** The VM itself prevents contract re-entry, making reentrancy attacks fundamentally impossible in the current Soroban execution model. The application-level guard (`acquire_lock`/`release_lock`) serves as defense-in-depth and ensures the contract remains safe even if the runtime behavior changes in future versions.

## Recommendations

| # | Recommendation | Status |
|---|---------------|--------|
| 1 | Keep the application-level guard — defense-in-depth | ✅ Done |
| 2 | Maintain CEI ordering — state updates before external calls | ✅ Done (hardened `withdraw_bond`, `withdraw_early`) |
| 3 | Restrict `set_callback` to admin only | ✅ Done — signature is now `set_callback(admin, callback)` |
| 4 | Add access control to `deposit_fees` | ⚠️ Open — currently unrestricted |
| 5 | Emit events on withdrawal/slash/fee-collect | ⚠️ Open — events are emitted via `emit_bond_withdrawn` but not for every path |
