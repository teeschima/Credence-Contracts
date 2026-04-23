# Bond Market Activation Checklist

Pre-activation checklist for the `credence_bond` contract. All items must pass before a bond market is considered safe to activate.

## 1. Token Configuration

- [ ] Bond token address is set (`set_token` / `set_bond_token`)
- [ ] Token contract is deployed and verified on the target network

## 2. Fee Configuration

- [ ] Fee treasury address is set (`set_fee_config`)
- [ ] Protocol fee bps is within bounds: `0 ≤ fee_bps ≤ 1000` (0 – 10 %)
- [ ] Attestation fee bps is within bounds: `0 ≤ fee_bps ≤ 500` (0 – 5 %)

## 3. Tier Threshold Ordering

All thresholds must be strictly increasing:

| Tier     | Min (tokens, 6 dec) | Max (tokens, 6 dec) | Constraint              |
|----------|---------------------|---------------------|-------------------------|
| Bronze   | 0                   | 1 000 000           | > 0                     |
| Silver   | 100                 | 10 000 000          | > bronze                |
| Gold     | 1 000               | 100 000 000         | > silver                |
| Platinum | 10 000              | 1 000 000 000       | > gold                  |

- [ ] `bronze_threshold > 0`
- [ ] `silver_threshold > bronze_threshold`
- [ ] `gold_threshold > silver_threshold`
- [ ] `platinum_threshold > gold_threshold`

## 4. Bond Amount Bounds

- [ ] Bond amount is positive (`amount > 0`)
- [ ] Bond amount is not negative
- [ ] Bond amount ≥ `MIN_BOND_AMOUNT` (1 000 stroops)
- [ ] Bond amount ≤ `MAX_BOND_AMOUNT` (100 000 000 000 000)
- [ ] Bond amount ≥ `bronze_threshold`

## 5. Duration Bounds

- [ ] Duration ≥ `MIN_BOND_DURATION` (86 400 s = 1 day)
- [ ] Duration ≤ `MAX_BOND_DURATION` (31 536 000 s = 365 days)

## 6. Leverage

- [ ] `max_leverage` is set and > 0
- [ ] `max_leverage` is within bounds: `1 ≤ max_leverage ≤ 100 000 000`
- [ ] `bond_amount / MIN_BOND_AMOUNT ≤ max_leverage`

## 7. Rolling Bond (if applicable)

- [ ] `notice_period_duration > 0`
- [ ] `notice_period_duration ≤ bond_duration`
- [ ] `notice_period_duration ≥ cooldown_period`

## 8. Emergency Configuration (if enabled)

- [ ] Governance address is set
- [ ] Treasury address is set
- [ ] `emergency_fee_bps ≤ 10 000` (≤ 100 %)

## 9. Supply Cap (if enforced)

- [ ] `supply_cap ≥ 0` (0 = no cap)
- [ ] Current `total_supply + net_amount ≤ supply_cap`

## 10. Regression Tests

Run before every deployment:

```sh
cargo test -p credence_bond
```

Key test coverage in `src/test_market_activation.rs`:

| Test | Validates |
|------|-----------|
| `test_activation_fails_without_token_config` | Token not set → panic |
| `test_activation_fails_with_zero_gold_threshold` | Gold ≤ silver → panic |
| `test_set_gold_threshold_above_max_panics` | Gold > MAX → setter panics |
| `test_activation_fails_with_platinum_equal_to_gold` | Platinum ≤ gold → panic |
| `test_set_platinum_threshold_above_max_panics` | Platinum > MAX → setter panics |
| `test_set_protocol_fee_bps_above_max_panics` | Fee bps > 1000 → setter panics |
| `test_set_protocol_fee_bps_zero_is_valid` | Fee bps = 0 is allowed |
| `test_activation_fails_with_duration_below_minimum` | Duration < 1 day → panic |
| `test_activation_fails_with_duration_above_maximum` | Duration > 365 days → panic |
| `test_activation_fails_with_negative_amount` | Negative amount → panic |
| `test_activation_fails_with_zero_amount` | Zero amount → panic |
| `test_valid_bond_activation_succeeds` | All params valid → bond active |
