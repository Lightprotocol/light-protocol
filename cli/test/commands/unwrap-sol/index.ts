import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { decompress } from "@lightprotocol/stateless.js";

class UnwrapSolCommand extends Command {
  static summary = "Unwrap SOL from compressed account.";

  static examples = ["$ light unwrap-sol --to PublicKey --amount 10"];

  static flags = {
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to unwrap, in lamports.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(UnwrapSolCommand);
    const to = flags["to"];
    const amount = flags["amount"];
    if (!to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing unwrap-sol...\n`);
    loader.start();

    try {
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      const txId = await decompress(rpc(), payer, amount, toPublicKey);
      loader.stop(false);
      console.log(
        "\x1b[32munwrap-sol:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("unwrap-sol successful");
    } catch (error) {
      this.error(`Failed to unwrap-sol!\n${error}`);
    }
  }
}

export default UnwrapSolCommand;
