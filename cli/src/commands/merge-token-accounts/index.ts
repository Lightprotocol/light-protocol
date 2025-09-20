import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getKeypairFromFile,
  rpc,
} from "../../utils/utils";
import { mergeTokenAccounts } from "@lightprotocol/compressed-token";
import { PublicKey } from "@solana/web3.js";

class MergeTokenAccountsCommand extends Command {
  static summary = "Merge all token accounts for a specific mint.";

  static examples = ["$ light merge-token-accounts --mint PublicKey"];

  static flags = {
    mint: Flags.string({
      description: "Mint to merge accounts for",
      required: true,
    }),
    "fee-payer": Flags.string({
      description:
        "Specify the fee-payer account. Defaults to the client keypair.",
      required: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(MergeTokenAccountsCommand);

    const loader = new CustomLoader(`Merging token accounts...\n`);
    loader.start();

    try {
      const mint = new PublicKey(flags.mint);

      let payer = defaultSolanaWalletKeypair();
      if (flags["fee-payer"]) {
        payer = await getKeypairFromFile(flags["fee-payer"]);
      }

      const txId = await mergeTokenAccounts(rpc(), payer, mint, payer);

      loader.stop(false);
      console.log(
        `\x1b[1mMerge tx:\x1b[0m `,
        generateSolanaTransactionURL("tx", txId, "custom"),
      );

      console.log("Token accounts merged successfully");
    } catch (error) {
      this.error(`Failed to merge token accounts!\n${error}`);
    }
  }
}

export default MergeTokenAccountsCommand;
