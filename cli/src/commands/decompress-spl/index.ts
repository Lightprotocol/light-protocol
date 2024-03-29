import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import { decompress } from "@lightprotocol/compressed-token";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

/// TODO: add ability to decompress from non-fee payer
class DecompressSplCommand extends Command {
  static summary = "Decompress into SPL tokens.";

  static examples = [
    "$ light decompress-spl --mint PublicKey --to PublicKey --amount 10",
  ];

  static flags = {
    mint: Flags.string({
      description: "Specify the mint address.",
      required: true,
    }),
    to: Flags.string({
      description:
        "Specify the recipient address. (owner of destination token account)",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to decompress, in tokens.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(DecompressSplCommand);
    const to = flags["to"];
    const mint = flags["mint"];
    const amount = flags["amount"];
    if (!to || !mint || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing decompress-spl...\n`);
    loader.start();
    let txId;
    try {
      const toPublicKey = new PublicKey(to);
      const mintPublicKey = new PublicKey(mint);
      const payer = defaultSolanaWalletKeypair();

      const rpc = createRpc(getSolanaRpcUrl());

      const recipientAta = await getOrCreateAssociatedTokenAccount(
        rpc,
        payer,
        mintPublicKey,
        toPublicKey,
      );

      txId = await decompress(
        rpc,
        payer,
        mintPublicKey,
        amount,
        payer,
        recipientAta.address,
      );

      loader.stop(false);
      console.log(
        "\x1b[32mtxId:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("decompress-spl successful");
    } catch (error) {
      console.log("decompress-spl failed", txId);
      this.error(`Failed to decompress-spl!\n${error}`);
    }
  }
}

export default DecompressSplCommand;
