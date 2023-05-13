import { Args, Command, Flags } from "@oclif/core";
import { getUser } from "../../utils";

class MergeUtxosCommand extends Command {
  static description = "Merge multiple UTXOs into a single UTXO";

  static flags = {
    latest: Flags.boolean({
      char: "l",
      description: "Use the latest UTXOs",
      default: false,
    }),
  };

  static args = {
    commitments: Args.string({
      name: "commitments",
      description: "Commitments of the UTXOs to merge",
      required: true,
      multiple: true,
    }),
    token: Args.string({
      name: "token",
      description: "Token of the UTXOs to merge",
      required: true,
    }),
  };

  static examples = [
    "$ light merge-utxos --latest --token USDC 0xcommitment1 0xcommitment2 0xcommitment3",
  ];

  async run() {
    const { flags, args } = await this.parse(MergeUtxosCommand);
    const { commitments, token } = args;
    const { latest } = flags;

    const user = await getUser();

    try {
      await user.mergeUtxos(commitments, token, latest);
      this.log("UTXOs merged successfully!");
    } catch (error) {
      this.error(`Error merging UTXOs: ${error}`);
    }
  }
}

module.exports = MergeUtxosCommand;
