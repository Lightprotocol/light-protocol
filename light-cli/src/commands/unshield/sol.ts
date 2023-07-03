import { Command, Flags, Args } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";

class UnshieldCommand extends Command {
  static summary = "Unshield SOL for a user";
  static usage = "unshield:sol <AMOUNT> <RECIPIENT_ADDRESS> [FLAGS]";
  static examples = [
    "$ light unshield:sol 5 <RECIPIENT_ADDRESS>",
  ];

  static flags = {
    'minimum-lamports': Flags.boolean({
      char: "m",
      description:
        "Whether to use the minimum required lamports for the unshield transaction",
      default: false,
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SOL amount to unshield",
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

    const loader = new CustomLoader("Performing token unshield...\n");
    loader.start();

    try {

      const user = await getUser();
      const response = await user.unshield({
        token: "SOL",
        recipient: new PublicKey(recipient),
        publicAmountSol: amountSol,
        minimumLamports,
      });

      this.log(generateSolanaTransactionURL("tx", `${response.txHash.signatures}`, "custom"));
      this.log(
        `\nSuccessfully unshielded ${amountSol} SOL`,
        "\x1b[32m✔\x1b[0m"
      );
      loader.stop();
    } catch (error) {
      this.error(`Failed to unshield SOL!\n${error}`);
    }
  }
}

export default UnshieldCommand;
