# Quickstart: Tree Fee Distribution

## Integration Test Scenarios

### Feature A1: Address Tree Fee Cap

```
Test: address tree forester receives min(5000, network_fee)
1. Initialize address tree with network_fee = 10000
2. Call update_address_merkle_tree (V1) or batch_update_address_tree (V2)
3. Assert forester received exactly 5000 (not 10000)
4. Assert tree/queue account decreased by 5000 (not 10000)
```

### Feature A2: State Tree Reimbursement PDA

```
Test: batch_append funds PDA, batch_nullify disburses
1. Initialize state tree with network_fee = 5000
2. Call init_reimbursement_pda for the tree
3. Call batch_append via registry
4. Assert forester net = +5000 (received 10000, sent 5000 to PDA)
5. Assert PDA balance increased by 5000
6. Call batch_nullify via registry
7. Assert forester received 5000 from PDA
8. Assert PDA balance decreased by 5000
```

```
Test: batch_nullify succeeds with empty PDA
1. Initialize state tree and reimbursement PDA
2. Call batch_nullify via registry (PDA has 0 beyond rent)
3. Assert nullification succeeded
4. Assert forester received 0 reimbursement
```

```
Test: nullify_leaves claws back excess to PDA
1. Initialize V1 state tree with network_fee = 10000
2. Call init_reimbursement_pda
3. Call nullify_leaves via registry
4. Assert forester net = +5000 (received 10000, sent 5000 to PDA)
```

### Feature B: Excess Fee Claiming

```
Test: claim_fees transfers excess to fee_recipient
1. Initialize state tree, accumulate fees via user transactions
2. Set protocol_fee_recipient in ProtocolConfig
3. Call claim_fees via registry
4. Assert tree balance = hardcoded_rent + rollover_fee * (capacity - next_index + 1)
5. Assert fee_recipient balance increased by the excess
```

```
Test: claim_fees with zero excess is a no-op
1. Initialize fresh tree (no accumulated fees beyond reserves)
2. Call claim_fees
3. Assert no transfer occurred
```

```
Test: network_fee == 0 skips all fee logic
1. Initialize tree with network_fee = 0
2. Call batch operations
3. Assert no reimbursement, no PDA transfers, no fee claiming
```

### Feature B: Negative Tests

```
Test: claim_fees fails with unregistered forester
1. Call claim_fees via registry with an unregistered signer
2. Assert transaction fails
```

```
Test: claim_fees fails on excluded account types
1. Call claim_fees with a V1 nullifier queue account
2. Assert transaction fails (InvalidAccountType)
3. Repeat with V1 address queue account
```

```
Test: claim_fees fee_recipient must match protocol config
1. Set protocol_fee_recipient in ProtocolConfig to address A
2. Call claim_fees with fee_recipient = address B
3. Assert transaction fails
```

```
Test: claim_fees on rolled-over tree
1. Initialize and roll over a state tree
2. Call claim_fees on the old (rolled-over) tree
3. Assert excess fees are still claimable from the old tree
```

### Feature B: Protocol Config

```
Test: set protocol_fee_recipient via update_protocol_config
1. Call update_protocol_config with protocol_fee_recipient = address A
2. Read ProtocolConfig
3. Assert protocol_fee_recipient == address A
```

## Running Tests

```bash
# Account-compression tests (claim_fees, address tree cap)
cargo test-sbf -p account-compression-test

# Registry tests (PDA init, batch_append/nullify PDA flow)
cargo test-sbf -p registry-test
```
