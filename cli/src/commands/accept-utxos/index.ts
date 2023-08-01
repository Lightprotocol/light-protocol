import { Command, Flags } from "@oclif/core";
import { TOKEN_REGISTRY, User, ConfirmOptions } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";
import { standardFlags } from "../../utils";

class MergeUtxosCommand extends Command {
  static description = "Merge multiple inbox utxos into a single UTXO.";
  static examples = [
    "$ light accept-utxos --token USDC --commitment-hashes <COMMITMENT1> <COMMITMENT2> <COMMITMENT3>",
    "$ light accept-utxos --latest --token USDC --all",
  ];
  static args = {};
  static flags = {
    ...standardFlags,
    latest: Flags.boolean({
      char: "l",
      description: "Use the latest utxos.",
      hidden: true,
      default: true,
    }),
    token: Flags.string({
      name: "token",
      char: "t",
      description: "Token of the utxos to merge.",
      parse: async (token) => token.toUpperCase(),
      required: true,
    }),
    all: Flags.boolean({
      char: "a",
      description: "Merge all inbox utxos of an asset.",
      default: false,
    }),
    "commitment-hashes": Flags.string({
      char: "c",
      description: "Commitment hashes of the utxos to merge.",
      multiple: true,
      required: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(MergeUtxosCommand);
    const { latest, token, all } = flags;
    const commitments = flags["commitment-hashes"];

    const loader = new CustomLoader("Performing UTXO merge...\n");
    loader.start();
    try {
      const user: User = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRelayer: flags["localTestRelayer"],
      });
      const tokenCtx = TOKEN_REGISTRY.get(token);

      let response;
      if (all) {
        response = await user.mergeAllUtxos(
          tokenCtx?.mint!,
          ConfirmOptions.spendable,
          latest
        );
      } else {
        if (!commitments)
          throw new Error(
            "Please provide commitment hashes to merge or use --all flag"
          );
        response = await user.mergeUtxos(
          commitments,
          tokenCtx?.mint!,
          ConfirmOptions.spendable,
          latest
        );
      }
      this.log(
        `\nTransaction signatures: ${response.txHash.signatures![0]} \n ${
          response.txHash.signatures![1]
        }`
      );
      this.log(
        generateSolanaTransactionURL(
          "tx",
          `${response.txHash.signatures![1]}`,
          "custom"
        )
      );
      this.log(`\nAccepted ${token} inbox utxos successfully \x1b[32m✔\x1b[0m`);
      loader.stop();
    } catch (error) {
      this.error(`\nFailed to accept ${token} inbox utxos!\n${error}`);
    }
  }
}

export default MergeUtxosCommand;
