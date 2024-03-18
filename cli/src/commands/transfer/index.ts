import { Args, Command, Flags } from "@oclif/core";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getPayer,
  getRpc,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { transfer } from "@lightprotocol/compressed-token";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { confirmTx } from "@lightprotocol/stateless.js";
class TransferCommand extends Command {
  static summary = "Transfer tokens from one account to another.";

  static examples = [
    "$ light transfer --mint PublicKey --to PublicKey --amount 1000",
  ];

  static flags = {
    mint: Flags.string({
      description: "Token to transfer",
      required: false,
    }),
    to: Flags.string({
      description: "Recipient address",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to send, in tokens",
      required: true,
    }),
    "fee-payer": Flags.string({
      description:
        "Specify the fee-payer account. Defaults to the client keypair.",
      required: false,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);

    const loader = new CustomLoader(`Performing mint-to...\n`);
    loader.start();

    try {
      const mint = flags["mint"];
      const to = flags["to"];
      const amount = flags["amount"];
      if (!mint || !to || !amount) {
        throw new Error("Invalid arguments");
      }

      const mintPublicKey = new PublicKey(mint);
      const toPublicKey = new PublicKey(to);

      const payer = getPayer();

      let feePayer = payer;
      if (flags["fee-payer"]) {
        const decoded = bs58.decode(flags["fee-payer"]);
        feePayer = Keypair.fromSecretKey(decoded);
      }
      const connection = new Connection(getSolanaRpcUrl());

      const txId = await transfer(
        connection,
        feePayer,
        mintPublicKey,
        amount,
        payer,
        toPublicKey,
      );
      loader.stop(false);
      console.log(
        "\x1b[1mTransfer tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );

      console.log("transfer successful");
    } catch (error) {
      this.error(`Failed to transfer!\n${error}`);
    }
  }
}

export default TransferCommand;
