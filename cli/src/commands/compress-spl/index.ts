import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import { compress } from "@lightprotocol/compressed-token";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

/// TODO: add ability to compress from non-fee payer
class CompressSplCommand extends Command {
  static summary = "Compress SPL tokens.";

  static examples = [
    "$ light compress-spl --mint PublicKey --to PublicKey --amount 10",
  ];

  static flags = {
    mint: Flags.string({
      description: "Specify the mint address.",
      required: true,
    }),
    to: Flags.string({
      description:
        "Specify the recipient address (owner of destination compressed token account).",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to compress, in tokens.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(CompressSplCommand);
    const to = flags["to"];
    const mint = flags["mint"];
    const amount = flags["amount"];
    if (!to || !mint || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing compress-spl...\n`);
    loader.start();
    let txId;
    try {
      const toPublicKey = new PublicKey(to);
      const mintPublicKey = new PublicKey(mint);
      const payer = defaultSolanaWalletKeypair();

      const rpc = createRpc(getSolanaRpcUrl());

      /// TODO: add explicit check that the ata is valid
      const sourceAta = getAssociatedTokenAddressSync(
        mintPublicKey,
        payer.publicKey,
      );

      txId = await compress(
        rpc,
        payer,
        mintPublicKey,
        amount,
        payer,
        sourceAta,
        toPublicKey,
      );

      loader.stop(false);
      console.log(
        "\x1b[32mtxId:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("compress-spl successful");
    } catch (error) {
      console.log("compress-spl failed", txId);
      this.error(`Failed to compress-spl!\n${error}`);
    }
  }
}

export default CompressSplCommand;
