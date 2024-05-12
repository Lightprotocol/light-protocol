import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { decompress, getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

class DecompressSolCommand extends Command {
  static summary = "Decompress SOL.";

  static examples = ["$ light decompress-sol --to PublicKey --amount 10"];

  static flags = {
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to decompress, in lamports.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(DecompressSolCommand);
    const to = flags["to"];
    const amount = flags["amount"];
    if (!to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing decompress-sol...\n`);
    loader.start();

    try {
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      const lightWasm = await WasmFactory.getInstance();
      const rpc = await getTestRpc(lightWasm);
      const txId = await decompress(rpc, payer, amount, toPublicKey);
      loader.stop(false);
      console.log(
        "\x1b[32mdecompress-sol:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("decompress-sol successful");
    } catch (error) {
      this.error(`Failed to decompress-sol!\n${error}`);
    }
  }
}

export default DecompressSolCommand;
