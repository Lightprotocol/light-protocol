import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { compress, getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

class CompressSolCommand extends Command {
  static summary = "Compress SOL.";

  static examples = ["$ light compress-sol --to PublicKey --amount 10"];

  static flags = {
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to compress, in lamports.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(CompressSolCommand);
    const to = flags["to"];
    const amount = flags["amount"];
    if (!to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing compress-sol...\n`);
    loader.start();
    let txId;
    try {
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      const lightWasm = await WasmFactory.getInstance();
      const rpc = await getTestRpc(lightWasm);
      txId = await compress(rpc, payer, amount, toPublicKey);

      loader.stop(false);
      console.log(
        "\x1b[32mtxId:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("compress-sol successful");
    } catch (error) {
      console.log("compress-sol failed", txId);
      this.error(`Failed to compress-sol!\n${error}`);
    }
  }
}

export default CompressSolCommand;
