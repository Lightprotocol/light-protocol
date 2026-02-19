import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { compress } from "@lightprotocol/stateless.js";

class WrapSolCommand extends Command {
  static summary = "Wrap SOL into compressed account.";

  static examples = ["$ light wrap-sol --to PublicKey --amount 10"];

  static flags = {
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to wrap, in lamports.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(WrapSolCommand);
    const to = flags["to"];
    const amount = flags["amount"];
    if (!to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing wrap-sol...\n`);
    loader.start();
    let txId;
    try {
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      txId = await compress(rpc(), payer, amount, toPublicKey);

      loader.stop(false);
      console.log(
        "\x1b[32mtxId:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("wrap-sol successful");
    } catch (error) {
      console.log("wrap-sol failed", txId);
      this.error(`Failed to wrap-sol!\n${error}`);
    }
  }
}

export default WrapSolCommand;
