import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { unwrap, CompressedTokenProgram } from "@lightprotocol/compressed-token";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

/// TODO: add ability to unwrap from non-fee payer
class UnwrapSplCommand extends Command {
  static summary = "Unwrap Light Tokens into SPL token account.";

  static examples = [
    "$ light unwrap-spl --mint PublicKey --to PublicKey --amount 10",
  ];

  static flags = {
    mint: Flags.string({
      description: "Specify the mint address.",
      required: true,
    }),
    to: Flags.string({
      description:
        "Specify the recipient address. (owner of destination SPL token account)",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to unwrap, in tokens.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(UnwrapSplCommand);
    const to = flags["to"];
    const mint = flags["mint"];
    const amount = flags["amount"];
    if (!to || !mint || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing unwrap-spl...\n`);
    loader.start();
    let txId;
    try {
      const toPublicKey = new PublicKey(to);
      const mintPublicKey = new PublicKey(mint);
      const payer = defaultSolanaWalletKeypair();
      const tokenProgramId = await CompressedTokenProgram.getMintProgramId(
        mintPublicKey,
        rpc(),
      );

      const recipientAta = await getOrCreateAssociatedTokenAccount(
        rpc(),
        payer,
        mintPublicKey,
        toPublicKey,
        undefined,
        undefined,
        undefined,
        tokenProgramId,
      );

      txId = await unwrap(
        rpc(),
        payer,
        recipientAta.address,
        payer,
        mintPublicKey,
        BigInt(amount),
      );

      loader.stop(false);
      console.log(
        "\x1b[32mtxId:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("unwrap-spl successful");
    } catch (error) {
      console.log("unwrap-spl failed", txId);
      this.error(`Failed to unwrap-spl!\n${error}`);
    }
  }
}

export default UnwrapSplCommand;
