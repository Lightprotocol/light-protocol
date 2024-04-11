import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { compressLamports, createRpc } from "@lightprotocol/stateless.js";

class MintToCommand extends Command {
  static summary = "Compress SOL.";

  static examples = ["$ light compress-sol --to PublicKey --amount 10"];

  static flags = {
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to mint, in SOL.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(MintToCommand);
    const to = flags["to"];
    const amount = flags["amount"];
    if (!to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing compress-sol...\n`);
    loader.start();

    try {
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      const rpc = createRpc(getSolanaRpcUrl());
      const txId = await compressLamports(
        rpc,
        payer,
        amount * LAMPORTS_PER_SOL,
        toPublicKey,
      );

      loader.stop(false);
      console.log(
        "\x1b[compress-sol:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("compress-sol successful");
    } catch (error) {
      this.error(`Failed to compress-sol!\n${error}`);
    }
  }
}

export default MintToCommand;
