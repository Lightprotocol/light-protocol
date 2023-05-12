./validator/solana-test-validator --reset --limit-ledger-size 500000000 --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i /home/ananas/test_light/light-protocol-onchain/light-system-programs/target/deploy/verifier_program_zero.so --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 /home/ananas/test_light/light-protocol-onchain/light-system-programs/target/deploy/merkle_tree_program.so --bpf-program GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8 /home/ananas/test_light/light-protocol-onchain/light-system-programs/target/deploy/verifier_program_two.so --bpf-program Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS /home/ananas/test_light/light-protocol-onchain/market-place-verifier/target/deploy/market_place_verifier.so

TODO:

- Change Utxos
- Fetch only current offers
- enforce Offer expiry
- make circuit secure
- solve blinding issue definitively
- check that listing is legit
- tests program:
  - X make offer from/and deposit
  - make offer from utxo
  - X take offer to utxo
  - take offer and withdraw
  - cancel offer and withdraw
  - X cancel offer to utxo
- tests circuit:
- unit tests helper functions:
  - for every static function in MarketPlaceClient

Less immediate TODO:

- make encrypted offers/listing
- fetch encrypted offers/listing
- counter offers

Build Process:

- macro-circom binary -> creates circom file from .light file
- run circuit build -> creates rust verifyingkey and
- run anchor build
