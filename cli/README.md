### v3 CLI

- meant to replicate all typical uses of the Light SDK that any UI or wallet would implement too.
- /cli/src/commands -> UI
- build with `npx tsc`

- ./build/cli.js <command> <params> -flag

COMMANDS

initial setup:

./build/cli.js balance

consecutive:
./build/cli.js shield --amount=3 --token=SOL

./build/cli.js unshield --amount=1 --token=SOL --recipient=ErAe2LmEKgBNCSP7iA8Z6B396yB6LGCUjzuPrczJYDbz

./build/cli.js transfer --amount=1 --token=SOL --shieldedRecipient=19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434 --encryptionPublicKey=LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b

### TODOs Swen

- [] enable SPL support
- [] implement relayer (/relayer) WIP, replace relayer 'mocks' in user class
- [] add test cases [e.g. merging >2 inUtxos, 10-in circuit, extend checkBalance() for multiple out-utxos]
- [] test-case with browser wallet instead of CLI
