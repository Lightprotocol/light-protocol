import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { transfer } from "@lightprotocol/compressed-token";
import { PublicKey } from "@solana/web3.js";
import { getKeypairFromFile } from "@solana-developers/helpers";
import { createRpc } from "@lightprotocol/stateless.js";

class TransferCommand extends Command {
  static summary = "Transfer tokens from one account to another.";

  static examples = [
    "$ light transfer --mint PublicKey --to PublicKey --amount 1000",
  ];

  static flags = {
    mint: Flags.string({
      description: "Mint to transfer",
      required: true,
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
    const { flags } = await this.parse(TransferCommand);

    const loader = new CustomLoader(`Performing transfer...\n`);
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

      let payer = defaultSolanaWalletKeypair();
      if (flags["fee-payer"] !== undefined) {
        payer = await getKeypairFromFile(flags["fee-payer"]);
      }
      const rpc = createRpc(getSolanaRpcUrl());

      const txId = await transfer(
        rpc,
        payer,
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
