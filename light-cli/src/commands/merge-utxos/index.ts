import { Args, Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";
import { TOKEN_REGISTRY, User } from "@lightprotocol/zk.js";

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
    all: Flags.boolean({
      name: "all-inbox",
      description: "merges all inbox utxos of a asset",
      default: false,
    }),
  };

  static args = {
    commitments: Args.string({
      name: "commitments",
      description: "Commitments of the UTXOs to merge",
      required: false,
      multiple: true,
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples = [
    "$ light merge-utxos --latest --token USDC 0xcommitment1 0xcommitment2 0xcommitment3",
    "$ light merge-utxos --latest --token USDC --all",
  ];

  async run() {
    const { flags, args } = await this.parse(MergeUtxosCommand);
    const { commitments } = args;
    const { latest, token, all } = flags;

    const loader = new CustomLoader("Performing UTXO merge...");

    loader.start();

    const user: User = await getUser();

    const tokenSymbol = token.toUpperCase();

    const tokenCtx = TOKEN_REGISTRY.get(tokenSymbol);

    try {
      let response;
      if (all) {
        response = await user.mergeAllUtxos(tokenCtx?.mint!, latest);
      } else {
        response = await user.mergeUtxos(
          [commitments!],
          tokenCtx?.mint!,
          latest
        );
      }
      this.log("\nUTXOs merged successfully!");
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`\nUTXO merge failed: ${error}`);
    }
  }
}

export default MergeUtxosCommand;
