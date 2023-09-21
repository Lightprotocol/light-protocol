#!/bin/bash

# program ids and names
program_ids=("2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86" "J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc" "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i" "DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj" "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6")
program_names=("verifier_program_two" "verifier_program_one" "verifier_program_zero" "verifier_program_storage" "merkle_tree_program")

# keypair
# keypair="<YOUR_KEYPAIR>"

# network url
url="testnet"

# deploying programs
for index in ${!program_ids[*]}
do
  echo "Deploying program with ID: ${program_ids[$index]}"
  # derive the program file path
  program_filepath="./target/deploy/${program_names[$index]}.so"
  solana program deploy --program-id ${program_ids[$index]} $program_filepath
  echo "Deployment of program with ID: ${program_ids[$index]} completed"
done
