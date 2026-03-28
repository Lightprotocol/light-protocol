# Instruction Contracts: Tree Fee Distribution

## New Instructions

### account-compression: `claim_fees`

Transfers excess accumulated fees from a tree/queue account to a fee_recipient.

**Accounts**:
| Name | Type | Mut | Signer | Description |
|------|------|-----|--------|-------------|
| authority | AccountInfo | no | yes | Registered program CPI authority or tree owner |
| registered_program_pda | Option\<Account\<RegisteredProgram\>\> | no | no | For CPI access control |
| merkle_tree_or_queue | AccountInfo | yes | no | The tree or queue account to claim from |
| fee_recipient | AccountInfo | yes | no | Receives excess fees |

**Logic**:
1. `check_signer_is_registered_or_authority` (existing security model)
2. Determine account type from discriminator (V1 state/address tree, V2 batched tree, V2 output queue)
3. Read `rollover_fee`, `capacity`, `next_index` from account metadata
4. Look up hardcoded rent exemption for account size
5. `claimable = account_lamports.checked_sub(rent_exemption)?.checked_sub(rollover_fee.checked_mul(capacity.checked_sub(next_index)?.checked_add(1)?)?)?`
6. If claimable > 0: `transfer_lamports(merkle_tree_or_queue, fee_recipient, claimable)`

**Errors**:
- `InvalidAccountType` -- account is not a supported tree/queue type
- `NoExcessFees` -- claimable is zero or negative (optional: could just succeed as no-op)

---

### registry: `claim_fees` (wrapper)

**Accounts**:
| Name | Type | Mut | Signer | Description |
|------|------|-----|--------|-------------|
| registered_forester_pda | Option\<Account\<ForesterEpochPda\>\> | yes | no | Forester eligibility |
| authority | Signer | no | yes | Forester |
| cpi_authority | AccountInfo | no | no | PDA for CPI signing |
| registered_program_pda | AccountInfo | no | no | For access control |
| account_compression_program | Program | no | no | CPI target |
| merkle_tree_or_queue | AccountInfo | yes | no | The tree/queue to claim from |
| protocol_config_pda | Account\<ProtocolConfigPda\> | no | no | Read fee_recipient |
| fee_recipient | AccountInfo | yes | no | Must match protocol_config.protocol_fee_recipient |

**Logic**:
1. Validate forester eligibility (existing `check_forester` pattern)
2. Verify `fee_recipient == protocol_config.protocol_fee_recipient`
3. CPI to account-compression `claim_fees`

---

### registry: `init_reimbursement_pda`

**Accounts**:
| Name | Type | Mut | Signer | Description |
|------|------|-----|--------|-------------|
| payer | Signer | yes | yes | Pays rent for PDA creation |
| reimbursement_pda | AccountInfo | yes | no | PDA to initialize, seeds: [b"reimbursement", tree.key()] |
| tree | AccountInfo | no | no | Must be valid state tree owned by account-compression |
| system_program | Program | no | no | For account creation |

**Logic**:
1. Validate tree is owned by account-compression program
2. Validate tree has a state tree discriminator. Accepted: V1 `StateMerkleTreeAccount` discriminator, V2 batched state tree discriminator (`TreeType::StateV2`). Rejected: address tree discriminators, queue discriminators.
3. Create PDA with Anchor `init` constraint, seeds = `[b"reimbursement", tree.key().as_ref()]`

---

## Modified Instructions

### account-compression: `update_address_merkle_tree` (V1)

**Change**: Line ~135, replace `network_fee` with `min(5000, network_fee)` in transfer_lamports call.

### account-compression: `batch_update_address_tree` (V2)

**Change**: Line ~74, replace `network_fee` with `min(5000, network_fee)` in transfer_lamports call.

### Transfer Mechanisms

- **Forester -> PDA**: `system_program::transfer` (forester is a signer). Requires `system_program` in account context.
- **PDA -> Forester**: Direct lamport manipulation (`**pda.lamports.borrow_mut() -= amount; **forester.lamports.borrow_mut() += amount`). Registry owns the PDA, so no CPI or signer seeds needed -- the runtime allows a program to debit accounts it owns. Alternatively, `invoke_signed` with PDA seeds.

### registry: `batch_append` (wrapper)

**Added accounts**: `reimbursement_pda` (mut), `system_program`
**Added logic**: After CPI, if `network_fee >= 5000`: `system_program::transfer(forester, reimbursement_pda, network_fee)`

### registry: `batch_nullify` (wrapper)

**Changed account**: `authority` changed to `#[account(mut)]` (to receive lamports)
**Added account**: `reimbursement_pda` (mut)
**Added logic**: After CPI, if PDA has >= rent_exempt + 5000: direct lamport transfer from PDA to authority (5000 lamports)

### registry: `nullify_leaves` (wrapper)

**Added accounts**: `reimbursement_pda` (mut), `system_program`
**Added logic**: After CPI, if `network_fee > 5000`: `system_program::transfer(forester, reimbursement_pda, network_fee - 5000)`
