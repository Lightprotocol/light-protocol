import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { Connection, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { Rpc, decompressLamports } from "@lightprotocol/stateless.js";

class MintToCommand extends Command {
  static summary = "Decompress SOL.";

  static examples = ["$ light decompress-sol --to PublicKey --amount 10"];

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

      const connection = new Connection(getSolanaRpcUrl());

      const txId = await decompressLamports(
        connection as Rpc,
        payer,
        amount * LAMPORTS_PER_SOL,
        toPublicKey,
      );
      loader.stop(false);
      console.log(
        "\x1b[decompress-sol:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("decompress-sol successful");
    } catch (error) {
      this.error(`Failed to decompress-sol!\n${error}`);
    }
  }
}

export default MintToCommand;
