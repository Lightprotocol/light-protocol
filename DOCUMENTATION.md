# Light Protocol Documentation

Light Protocol is privacy infrastructure on Solana. The core of the protocol is a shielded pool implemented in a Solana program. The shielded pool is basically Zcash in a smart contract (Solana program). The shielded pool\`s ZKPs are generated with the Groth16 proving system, implemented over the bn254 curve. The circuit of the shielded pool is implemented in [light-pool-client](https://github.com/Lightprotocol/light-pool-client/Light_circuits/circuits). The Merkle tree used in the current implementation is of height 18 and uses poseidon hashes, implemented over the bn254 curve. Nullifiers and Merkle tree leaves are stored in individual accounts as described below.

### **Accounts**

Solana programs are stateless. State is stored in accounts. Accounts which are owned (and derived) by programs are called programs derived accounts (pdas). Light Protocol stores state in five accounts: a tmp_storage_pda, a merkle_tree_storage_pda, nullifier_pdas, leaves_pdas, and user_accounts.

**State Accounts:**

**tmp_storage_account:**

- stores the state of a shielded pool transaction
- only has to exist temporary for the computation
- account_id: 1
- rent exempt: false
- size: 3900

**merkle_tree_storage_account:**

- stores the state of a sparse Merkle tree
- tree height 18
- account_id: 2
- rent exempt: true
- size: 16658

**nullifier_pda:**

- is derived from a nullifier plus nullifier domain separation
- stores whether it is initalized and the account type
- account_id: 3
- rent exempt: true
- size: 2

**leaves_pda:**

- stores two Merkle tree leaves plus the public key of the respective Merkle tree
- account_id: 4
- rent exempt: true
- size: 106

**user_account:**

- stores encrypted user utxos
- can only be modified by the signer account which initialized the user account
- account_id: 5
- rent exempt: true
- size: 34 + UTXO_SIZE * UTXO_CAPACITY

    (currently UTXO_SIZE = 216 and UTXO_CAPACITY = 100)


**Token Accounts:**

**user_pda_token:**

- spl token account of the user
- deposit: funds are transferred from this account
- withdrawal: funds are transferred to this account

**relayer_pda_token:**

- spl token account of the relayer
- receives fees

**merkle_tree_pda_token:**

- spl token account of the Merkle tree
- this is the pool account to which tokens are transferred to at deposit and transferred from at withdrawal

**authority:**

- hardcoded authority account which is required to withdraw from merkle_tree_pda_token account

## Instructions

The Light Protocol program accepts 4 types of instructions initialized new merkle tree account, initialize new onchain user account, modifiy onchain user account, close onchain user account and transact with shielded pool.

### Initialize merkle tree account

Initializes a new Merkle tree account by copying hardcoded bytes into the new Merkle tree account. The hardcoded bytes are defined in config.rs.

initialize_merkle_tree_account_selector = 240u8
**instruction_data:** [ 8_bytes_buffer + initialize_merkle_tree_account_selector ]

**Accounts:**

1. signer
2. merkle_tree_storage_pda
3. rent_sysvar_info

New initialization bytes can be generated for Merkle trees of arbitrary heights  with cargo test merkle_tree_print_init_data_and_instruction_order.

### Initialize user account

Initializes a new user account which was created in a different instruction. The signer public key is stored as the account`s authority.

initialize_user_account_selector = 100u8

**instruction_data:** [ 8_bytes_buffer + initialize_user_account_selector ]

**Accounts:**

1. signer
2. user account
3. rent_sysvar_info

### **Modify user account**

Stores an arbitrary number of encrypted utxos at speficied indices. Existing utxo data at the index is overwritten.

modify_user_account_selector = 101u8
**instruction_data:** [ 8_bytes_buffer + modify_user_account_selector + (index , encrypted_utxo_bytes) + ... + (index , encrypted_utxo_bytes) ]

**Accounts:**

1. signer
2. user account
3. rent_sysvar_info

### Close user account

Closes a user account by transferring all of the user_account lamports to the invoking signer account.

close_user_account_selector = 102u8
**instruction_data:** 8_bytes_buffer + close_user_account_selector

**Accounts:**

1. signer
2. user account
3. rent_sysvar_info

### Transact with shielded pool

A complete a shielded pool transaction consists out of 1502 instructions. These instructions are the default path and do not require a selector. The index of the current computational step is stored in the tmp_storage_account and incremented at the end of every instruction. The passed-in instruction data and accounts vary depending on the instruction index. There are five different phases in the following order the send_data_instruction, check_root_instruction, ZKP_verification_instructions, merkle_tree_insert_instructions, and the last instruction.

**send_data_instruction:**

The first instruction sends in all data required for the computation, creates and initializes the tmp_storage_account, saves the data in the tmp_storage_account, and checks the external data hash.

**instruction_data:** [ 9_bytes_buffer +

root,
public amount,
external data hash,
nullifier0,
nullifier1,
leaf_right,
leaf_left,
proof,
recipient,
ext_amount,
relayer,
fee ]

**Accounts:**

1. signer
2. tmp_storage_pda
3. system_program_id
4. rent_sysvar_info

**check_root_instruction:**

Searches the Merkle tree root history array for the Merkle tree root of the ZKP.
**instruction_data:** none

**Accounts:**

1. signer
2. tmp_storage_pda
3. merkle_tree_storage_pda

**ZKP_verification_instructions:**

Perform ZKP verification.
**instruction_data:** none

**Accounts:**

1. signer
2. tmp_storage_pda

**merkle_tree_insert_instructions:**

Calculates a new Merkle tree root by inserting two new leaves. The leaf hashes are the commitment hashes of the output utxos. The first of the merkle_tree_insert_instructions locks the Merkle tree to prevent a race condition of several transactions trying to update the Merkle tree at the same time. The root and new leaves are not inserted in this instruction phase but in the last_instruction. Thus, the Merkle tree is only updated once all checks of the shielded transaction are passed successfully. This approach prevents corruption of the Merkle tree after
**instruction_data:** none

**Accounts:**

1. signer
2. tmp_storage_pda
3. merkle_tree_storage_pda

**last_instruction:**

Checks and inserts nullifiers, checks external amount, transfers tokens, transfers fees, inserts new merkle root.

Nullifiers are checked by trying to create pda accounts derived from the respective nullifier. If the account creation fails the nullifier already exists.

A positive external amount means the shielded transaction is a deposit. In this case tokens are transferred from a user_token_pda to the merkle_tree_token_pda.

A negative external amount which is equal to relayer fees means the transaction is an internal shielded pool transfer. In this case only the fees are transferred to the relayer.

A negative external amount greater than the relayer fees result mean the transaction is a withdrawal. The withdrawal token amount is transferred from the merkle_tree_token_pda to the user_token_account. After that, fees are transferred to the relayer.

At the end of the instruction the new Merkle tree root is inserted into the Merkle tree and the lock is released.

**instruction_data:** none

**Accounts:**

1. signer
2. tmp_storage_pda
3. two_leaves_pda
4. nullifier0_pda
5. nullifier1_pda
6. merkle_tree_pda
7. merkle_tree_pda_token
8. spl_program
9. token_program_account
10. rent_sysvar_info
11. authority
12. user_pda_token
13. relayer_pda_token
