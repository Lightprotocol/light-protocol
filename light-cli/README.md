# LIGHT CLI

## Installation

To use LIGHT CLI, you need to have Node.js (version 12 or later) and npm (Node Package Manager) installed on your machine. Follow the steps below to install [Your CLI Name] globally:

1. Open your terminal or command prompt.
2. Run the following command:

   ```shell
   npm install -g @lightprotocol/cli
   ```

3. After the installation is complete, you can verify the installation by running:

   ```shell
   light --version
   ```

   This should display the version number of [Your CLI Name].

## Usage

To use light, open your terminal or command prompt and run the following command:

```shell
light account
```

Replace `[command]` with the specific command you want to execute, and `[options]` with any additional options or flags supported by the command.

For detailed usage and available commands, you can use the `--help` flag:

```shell
light --help
```

This will display the list of available commands and their respective usage information.

### Commands

- `help`: Displays help information.
- `init`: Initializes a new Light project.
- `build`: Builds the Light project.
- `test`: Runs the Light Project tests.
- `account`: Get the current account details
- `airdrop`: Perform a native Solana or SPL airdrop to a user
- `balance`: Retrieve the balance, inbox balance, or UTXOs for the user
- `config`: Update the configuration values
- `history`: Retrieve transaction history for the user
- `merge-utxos`: Merge multiple UTXOs into a single UTXO
- `setup`: Perform setup tasks
- `shield`: Shield tokens for a user
- `transfer`: Transfer tokens to a recipient
- `unshield`: Unshield tokens for a user
- merkleTree:
  - `mt:authority`: Initialize, set, or get the Merkle Tree Authority
  - `mt:configuration`: Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration
  - `mt:initialize`: Initialize the Merkle Tree
  - `mt:pool`: Register a new pool type [default, spl, sol]
  - `mt:verifier`: Register a new verifier for a Merkle Tree

## License

light is released under the [Your License Name] License. See the LICENSE file for more details.

## Contact

If you have any questions, suggestions, or feedback, please feel free to reach out to us at [your-email@example.com].

Happy lighting!
