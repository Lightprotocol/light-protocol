import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getKeypairFromFile,
  rpc,
} from "../../utils/utils";
import { Keypair, PublicKey } from "@solana/web3.js";
import { approveAndMintTo } from "@lightprotocol/compressed-token";

class ApproveAndMintToCommand extends Command {
  static summary =
    "Mint tokens to a compressed account via external mint authority";

  static examples = [
    "$ light approve-and-mint-to --mint PublicKey --to PublicKey --amount 1000",
  ];

  static flags = {
    "mint-authority": Flags.string({
      description:
        "Specify the filepath of the mint authority keypair. Defaults to your local solana wallet.",
      required: false,
    }),
    mint: Flags.string({
      description: "Specify the mint address.",
      required: true,
    }),
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to mint, in tokens.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(ApproveAndMintToCommand);
    const { mint, to, amount } = flags;
    if (!mint || !to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing approve-and-mint-to...\n`);
    loader.start();

    try {
      const mintPublicKey = new PublicKey(mint);
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();
      let mintAuthority: Keypair = payer;
      if (flags["mint-authority"] !== undefined) {
        mintAuthority = await getKeypairFromFile(flags["mint-authority"]);
      }

      const txId = await approveAndMintTo(
        rpc(),
        payer,
        mintPublicKey,
        toPublicKey,
        mintAuthority,
        amount,
      );
      loader.stop(false);
      console.log(
        "\u001B[1mMint tx:\u001B[0m",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("approve-and-mint-to successful");
    } catch (error) {
      this.error(`Failed to approve-and-mint-to!\n${error}`);
    }
  }
}

export default ApproveAndMintToCommand;
