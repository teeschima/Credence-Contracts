# Event Indexing Migration Guide

## Overview

This document outlines the migration from v1 to v2 position lifecycle events in the Credence Contracts, which improves indexing capabilities for off-chain analytics and query efficiency.

## Problem Statement

The original position lifecycle events (`bond_created`, `bond_withdrawn`, `bond_increased`, `bond_slashed`) had suboptimal indexing that made off-chain queries expensive and error-prone:

- Only `identity` (user address) was indexed
- Critical fields like `amount`, `timestamp`, and `balance` were only in data payload
- No efficient way to filter by amount ranges or time periods
- Required full event data scanning for common analytics queries

## Solution: V2 Events with Enhanced Indexing

### New Event Structure

#### `bond_created_v2`
**Indexed Topics:**
- `Symbol` - "bond_created_v2"
- `Address` - The identity owning the bond
- `i128` - The initial bonded amount (now indexed!)
- `u64` - The bond start timestamp (now indexed!)

**Data:**
- `u64` - The duration of the bond in seconds
- `bool` - Whether the bond is rolling
- `u64` - Bond end timestamp (calculated)

#### `bond_withdrawn_v2`
**Indexed Topics:**
- `Symbol` - "bond_withdrawn_v2"
- `Address` - The identity owning the bond
- `i128` - The amount withdrawn (now indexed!)
- `i128` - The remaining bonded amount (now indexed!)
- `u64` - The withdrawal timestamp (now indexed!)

**Data:**
- `bool` - Whether this was an early withdrawal (penalty applied)
- `i128` - Penalty amount if early withdrawal

#### `bond_increased_v2`
**Indexed Topics:**
- `Symbol` - "bond_increased_v2"
- `Address` - The identity owning the bond
- `i128` - The additional amount added (now indexed!)
- `i128` - The new total bonded amount (now indexed!)
- `u64` - The increase timestamp (now indexed!)

**Data:**
- `bool` - Whether this increase crossed a tier threshold
- `BondTier` - New bond tier after increase

#### `bond_slashed_v2`
**Indexed Topics:**
- `Symbol` - "bond_slashed_v2"
- `Address` - The identity owning the bond
- `i128` - The amount slashed in this event (now indexed!)
- `i128` - The new total slashed amount for this bond (now indexed!)
- `u64` - The slash timestamp (now indexed!)
- `Address` - The admin who performed the slash (now indexed!)

**Data:**
- `String` - Reason for the slash
- `bool` - Whether this was a full slash (bond completely liquidated)

## Migration Strategy

### Backward Compatibility

During the migration period, both v1 and v2 events are emitted simultaneously:

```rust
// Emit both old and new events for backward compatibility during migration
events::emit_bond_created(&e, &identity, amount, duration, is_rolling);
events::emit_bond_created_v2(&e, &identity, amount, duration, is_rolling, bond_start);
```

### Indexer Migration Path

1. **Phase 1: Dual Event Processing**
   - Process both v1 and v2 events
   - Validate data consistency between versions
   - Build v2 indexing infrastructure

2. **Phase 2: V2 Priority**
   - Prioritize v2 events for new data
   - Use v1 events only for historical data
   - Implement fallback mechanisms

3. **Phase 3: V2 Complete**
   - Deprecate v1 event processing
   - Remove v1 event emission (future version)
   - Full v2 indexing utilization

### Query Improvements

#### Before (V1)
```javascript
// Inefficient - requires scanning all events
const largeBonds = events.filter(event => {
  if (event.topics[0] === 'bond_created') {
    const data = parseEventData(event.data);
    return data.amount >= 10000;
  }
});
```

#### After (V2)
```javascript
// Efficient - uses indexed amount field
const largeBonds = events.filter(event => {
  return event.topics[0] === 'bond_created_v2' && 
         event.topics[2] >= 10000; // Indexed amount
});
```

## Benefits

### For Off-Chain Indexers

1. **Reduced Computational Cost**
   - Filter by amount without parsing event data
   - Time-range queries using indexed timestamps
   - Balance queries using indexed remaining amounts

2. **Improved Query Performance**
   - Database indexes on frequently queried fields
   - Complex queries become simple indexed lookups
   - Support for real-time analytics dashboards

3. **Enhanced Analytics Capabilities**
   - Amount distribution analysis
   - Time-based trend analysis
   - Tier progression tracking
   - Slash pattern analysis

### For Smart Contract Users

1. **No Breaking Changes**
   - All existing functionality preserved
   - Gradual migration path
   - Backward compatible event emission

2. **Better Data Availability**
   - More detailed event information
   - Additional context (tier changes, penalties)
   - Improved audit trails

## Implementation Details

### Event Emission Pattern

```rust
// In contract functions
pub fn create_bond_with_rolling(...) -> IdentityBond {
    // ... bond creation logic ...
    
    // Emit both old and new events for backward compatibility during migration
    events::emit_bond_created(&e, &identity, amount, duration, is_rolling);
    events::emit_bond_created_v2(&e, &identity, amount, duration, is_rolling, bond_start);
    
    bond
}
```

### Testing Strategy

The migration includes comprehensive tests:

1. **Backward Compatibility Tests**
   - Verify both v1 and v2 events are emitted
   - Validate data consistency between versions
   - Test existing functionality unchanged

2. **Indexing Efficiency Tests**
   - Test amount-based filtering using indexed fields
   - Test time-based queries using indexed timestamps
   - Verify query performance improvements

3. **Schema Validation Tests**
   - Validate v2 event structure
   - Test data field accuracy
   - Ensure proper type handling

## Timeline

- **Week 1-2**: Implement v2 events and dual emission
- **Week 3-4**: Indexer migration and testing
- **Week 5-6**: Production deployment and monitoring
- **Week 7-8**: Performance validation and optimization

## Risk Mitigation

1. **Data Consistency**
   - Comprehensive test coverage
   - Data validation between v1 and v2 events
   - Rollback procedures if issues detected

2. **Indexer Compatibility**
   - Gradual migration path
   - Fallback mechanisms
   - Extensive testing with indexer teams

3. **User Impact**
   - No breaking changes to existing functionality
   - Clear communication about migration
   - Documentation and support

## Future Considerations

1. **V1 Event Deprecation**
   - Plan for eventual removal of v1 events
   - Communication timeline for indexer teams
   - Clean-up of deprecated code

2. **Additional Event Enhancements**
   - Consider adding more indexed fields based on usage patterns
   - Evaluate other contract events for similar improvements
   - Standardize event indexing patterns across contracts

3. **Performance Monitoring**
   - Track query performance improvements
   - Monitor indexer resource usage
   - Collect feedback from analytics teams

## Conclusion

The v2 event indexing improvements provide significant benefits for off-chain analytics while maintaining full backward compatibility. The gradual migration approach ensures minimal risk while delivering immediate value to indexers and analytics consumers.

The enhanced indexing capabilities enable more sophisticated analytics, better user experiences, and reduced infrastructure costs for off-chain data processing.
