1. │ ERROR: Not enough accounts. Requested 'burn change account owner' at index 3 but only 0 accounts available. programs/compressed-token/program/src/mint_action/burn.rs:142:41
   - means that packed accounts doesn't contain enough accounts
   -
2. `NotEnoughSigners`
   - `create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)`
   - needs more signers
   - signers must be unique you must not pass the same twice it will result in an error
3. `CompressedAccountError::ZeroCopyExpectedAddress => 12017`
   - when setting output compressed accounts in a zero copy we expect an address to be provided the address is allocated as Some by the ZeroCopyConfig but None is provided
   - any error that contains Expected and is an CompressedAccountError means this for the specied compressed account field
4. `Signer/Program cannot write into an account it doesn't own.`
    ```mode Small
      │ Signer/Program cannot write into an account it doesn't own. Write access check failed, compressed account owner [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] !=  invoking_program_id [9, 21, 163, 87, 35, 121, 78, 143, 182, 93, 7, 91, 107, 114, 105, 156, 56, 221, 2, 229, 148, 139, 117, 176, 229, 160, 65, 142, 128, 151, 91, 68].
      │ Program SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7 consumed 17422 of 1186879 compute units
      │ Program SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7 failed: custom program error: 0x177d
      │ Program cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m consumed 230543 of 1400000 compute units
      │ Program cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m failed: custom program error: 0x177d
    ```
    - the compressed output account owner is not set
5. ` Program SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7 failed: custom program error: 0x179a`
    -
    - the index for a state Merkle tree in the packed accounts is wrong
