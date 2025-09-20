import { Command, Flags } from "@oclif/core";
import { CustomLoader, rpc } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";

class BalanceCommand extends Command {
  static summary = "Get compressed SOL balance";
  static examples = ["$ light balance --owner=<ADDRESS>"];

  static flags = {
    owner: Flags.string({
      description: "Address of the owner.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(BalanceCommand);
    const loader = new CustomLoader(`Performing balance...\n`);
    loader.start();
    try {
      const { owner } = flags;
      const refOwner = new PublicKey(owner);

      const accounts = await rpc().getCompressedAccountsByOwner(refOwner);

      loader.stop(false);

      if (accounts.items.length === 0) {
        console.log("No accounts found");
        return;
      }

      let totalAmount = 0;
      for (const account of accounts.items) {
        totalAmount += account.lamports.toNumber();
      }

      console.log(
        "\u001B[1mCompressed SOL balance:\u001B[0m",
        totalAmount.toString(),
      );
    } catch (error) {
      this.error(`Failed to get balance!\n${error}`);
    }
  }
}

export default BalanceCommand;
