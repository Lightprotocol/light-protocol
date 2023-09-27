import { Command, Args } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils";
import { shieldSolFlags } from ".";
import { confirmOptionsFlags, standardFlags } from "../../utils";

class ShieldSolCommand extends Command {
  static summary = "Shield SOL for a user.";
  static usage = "shield:SOL <AMOUNT> [FLAGS]";
  static examples = [
    "$ light shield:SOL 1.3 --recipient <SHIELDED_RECIPIENT_ADDRESS> ",
    "$ light shield:SOL 12345678 -d",
  ];

  static flags = {
    ...standardFlags,
    ...standardFlags,
    ...shieldSolFlags,
    ...confirmOptionsFlags,
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SOL amount to shield.",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(ShieldSolCommand);
    const amountSol = args.amount;

    const recipient = flags["recipient"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];

    const loader = new CustomLoader("Performing shield operation...\n");
    loader.start();

    try {
      const user: User = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRelayer: flags["localTestRelayer"],
      });

      const response = await user.shield({
        token: "SOL",
        recipient,
        publicAmountSol: amountSol,
        minimumLamports: false,
        skipDecimalConversions,
        confirmOptions: getConfirmOptions(flags),
      });
      this.log(
        generateSolanaTransactionURL(
          "tx",
          `${response.txHash.signatures}`,
          "custom"
        )
      );
      const amount = skipDecimalConversions
        ? Number(amountSol) / 1_000_000_000
        : amountSol;
      this.log(`\nSuccessfully shielded ${amount} SOL`, "\x1b[32m✔\x1b[0m");
      loader.stop();
    } catch (error) {
      this.error(`Shielding tokens failed!\n${error}`);
    }
  }
}

export default ShieldSolCommand;
