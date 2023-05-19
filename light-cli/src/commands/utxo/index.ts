import { Args, Command, Flags } from "@oclif/core";
import { generateSolanaTransactionURL, getLoader, getUser } from "../../utils";
import { TOKEN_REGISTRY, User } from "light-sdk";

class MergeUtxosCommand extends Command {
  static description = "Merge multiple UTXOs into a single UTXO";

  static flags = {
    latest: Flags.boolean({
      char: "l",
      description: "Use the latest UTXOs",
      default: true,
    }),
    token: Flags.string({
      name: "token",
      description: "Token of the UTXOs to merge",
      required: true,
    }),
  };

  static args = {
    commitments: Args.string({
      name: "commitments",
      description: "Commitments of the UTXOs to merge",
      required: true,
      multiple: true,
    }),
  };

  static examples = [
    "$ light merge-utxos --latest --token USDC 0xcommitment1 0xcommitment2 0xcommitment3",
  ];

  async run() {
    const { flags, args } = await this.parse(MergeUtxosCommand);
    const { commitments } = args;
    const { latest, token } = flags;

    const { loader, end } = getLoader("Performing UTXO merge...");

    const user: User = await getUser();

    const tokenSymbol = token.toUpperCase();
    const tokenCtx = TOKEN_REGISTRY.get(tokenSymbol);

    try {
      const response = await user.mergeUtxos(
        [commitments],
        tokenCtx?.mint!,
        latest
      );

      this.log("UTXOs merged successfully!");
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`UTXO merge failed: ${error}`);
    }
  }
}

export default MergeUtxosCommand;
