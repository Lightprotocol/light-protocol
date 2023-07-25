
#!/usr/bin/env sh

declare -A keys=(
    ["merkle_tree_program"]="9VtiN5ibfgg27WaxJNpbu23VcsETt6LiirPBcsVXjpsc"
    ["verifier_program_zero"]="J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
    ["verifier_program_one"]="J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"
    ["verifier_program_two"]="2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"
    ["verifier_program_storage"]="2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"
)
mkdir -p bin/programs

for key in "${!keys[@]}"; do
    cp ../light-system-programs/target/deploy/$key.so ./bin/programs/$key.so
done
