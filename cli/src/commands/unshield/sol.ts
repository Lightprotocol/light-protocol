import { Command, Flags, Args } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils/utils";
import { confirmOptionsFlags, standardFlags } from "../../utils";

class UnshieldCommand extends Command {
  static summary = "Decompress SOL for a user.";
  static usage = "decompress:SOL <AMOUNT> <RECIPIENT_ADDRESS> [FLAGS]";
  static examples = ["$ light decompress:SOL 5 <RECIPIENT_ADDRESS>"];

  static flags = {
    ...standardFlags,
    ...confirmOptionsFlags,
    "minimum-lamports": Flags.boolean({
      char: "m",
      description:
        "Whether to use the minimum required lamports for the decompress transaction.",
      default: false,
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SOL amount to decompress.",
      required: true,
    }),
    recipient_address: Args.string({
      name: "RECIPIENT_ADDRESS",
      description: "The SOL account address of recipient.",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(UnshieldCommand);
    const amountSol = args.amount;
    const recipient = args.recipient_address;
    const minimumLamports = flags["minimum-lamports"];

    const loader = new CustomLoader("Performing token decompress...\n");
    loader.start();

    try {
      const user = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRpc: flags["localTestRpc"],
      });

      const response = await user.decompress({
        token: "SOL",
        recipient: new PublicKey(recipient),
        publicAmountSol: amountSol,
        minimumLamports,
        confirmOptions: getConfirmOptions(flags),
      });

      this.log(
        generateSolanaTransactionURL(
          "tx",
          `${response.txHash.signatures}`,
          "custom",
        ),
      );
      this.log(
        `\nSuccessfully decompressed ${amountSol} SOL`,
        "\x1b[32m✔\x1b[0m",
      );
      loader.stop();
    } catch (error) {
      this.error(`Failed to decompress SOL!\n${error}`);
    }
  }
}

export default UnshieldCommand;
