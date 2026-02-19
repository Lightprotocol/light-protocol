import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import {
  wrap,
  CompressedTokenProgram,
  getAssociatedTokenAddressInterface,
  createAtaInterfaceIdempotent,
} from "@lightprotocol/compressed-token";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

/// TODO: add ability to wrap from non-fee payer
class WrapSplCommand extends Command {
  static summary = "Wrap SPL tokens into Light Token account.";

  static examples = [
    "$ light wrap-spl --mint PublicKey --to PublicKey --amount 10",
  ];

  static flags = {
    mint: Flags.string({
      description: "Specify the mint address.",
      required: true,
    }),
    to: Flags.string({
      description:
        "Specify the recipient address (owner of destination Light Token account).",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to wrap, in tokens.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(WrapSplCommand);
    const to = flags["to"];
    const mint = flags["mint"];
    const amount = flags["amount"];
    if (!to || !mint || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing wrap-spl...\n`);
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

      const sourceAta = getAssociatedTokenAddressSync(
        mintPublicKey,
        payer.publicKey,
        false,
        tokenProgramId,
      );

      await createAtaInterfaceIdempotent(rpc(), payer, mintPublicKey, toPublicKey);
      const destAta = getAssociatedTokenAddressInterface(mintPublicKey, toPublicKey);

      txId = await wrap(
        rpc(),
        payer,
        sourceAta,
        destAta,
        payer,
        mintPublicKey,
        BigInt(amount),
      );

      loader.stop(false);
      console.log(
        "\x1b[32mtxId:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("wrap-spl successful");
    } catch (error) {
      console.log("wrap-spl failed", txId);
      this.error(`Failed to wrap-spl!\n${error}`);
    }
  }
}

export default WrapSplCommand;
