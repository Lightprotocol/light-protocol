# Light CLI

Official CLI to interact with Light Protocol v3 and build Private Solana Programs (https://github.com/Lightprotocol/light-protocol)

## Installation

To use Light CLI, you need to have Node.js (version 12 or later) and npm (Node Package Manager) installed on your machine. Follow the steps below to install [Your CLI Name] globally:

1. Open your terminal or command prompt.
2. Run the following command:

   ```shell
   npm install -g @lightprotocol/cli
   ```

3. After the installation is complete, you can verify the installation by running:

   ```shell
   light --version
   ```

## Usage

The CLI lets you initialize a PSP scaffold project and customize, build, and test it. (`light psp:...` subcommand)

It also provides utilities for local development, such as running a local Solana-test-validator with all necessary Light accounts and pre-loaded programs. (`light test-validator`).

You can also execute common user actions such as shielding and sending private transfers. (`light compress`, `light transfer`, `light decompress`).

For the full list of available commands, see below:

### Commands

- `help`: Display help information.
- `account`: Get the current account details
- `airdrop`: Perform a native Solana or SPL airdrop to a user.
- `balance`: Retrieve the balance, inbox balance, or utxos for the user.
- `config`: Update the configuration values.
- `history`: Retrieve transaction history for the user.
- `accept-utxos`: Merge multiple utxos into a single UTXO.
- `compress`: Compress tokens for a user.
  - `compress:sol`: Compress sol for a user.
  - `compress:spl`: Compress spl tokens for a user.
- `transfer`: Transfer tokens to a recipient.
- `decompress`: Decompress tokens for a user.

  - `decompress:sol`: Decompress sol for a user.
  - `decompress:spl`: Decompress spl tokens for a user.

- `test-validator`: Starts a solana-test-validator and with an initialized light environment. Use in a separate terminal instead of solana-test-validator.

- merkleTree:

  - `mt:authority`: Initialize, set, or get the Merkle Tree Authority.
  - `mt:configuration`: Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration.
  - `mt:initialize`: Initialize the Merkle Tree.
  - `mt:pool`: Register a new pool type [default, spl, sol].
  - `mt:verifier`: Register a new verifier for a Merkle Tree.

- psp:
  - `psp:init`: Initialize, set, or get the Merkle Tree Authority
  - `psp:build`: Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration.
  - `psp:test`: Perform the PSP tests.

## PSP Guide

You can find a comprehensive tutorial for building a custom Private Solana Program [here](https://docs.lightprotocol.com/getting-started/creating-a-custom-psp).

## License

@lightprotocol/cli is released under the GNU General Public License v3.0. See the LICENSE file for more details.

## Contact

If you have questions, suggestions, or feedback, join the developer community on [Discord](https://discord.gg/J3KvDfZpyp), or reach out at hello[at]lightprotocol[dot]com.
