import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getKeypairFromFile,
  rpc,
} from "../../utils/utils";
import { Keypair, PublicKey } from "@solana/web3.js";
import { mintTo } from "@lightprotocol/compressed-token";

class MintToCommand extends Command {
  static summary = "Mint tokens to an account.";

  static examples = [
    "$ light mint-to --mint PublicKey --to PublicKey --amount 1000",
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
    const { flags } = await this.parse(MintToCommand);
    const mint = flags["mint"];
    const to = flags["to"];
    const amount = flags["amount"];
    if (!mint || !to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing mint-to...\n`);
    loader.start();

    try {
      const mintPublicKey = new PublicKey(mint);
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      let mintAuthority: Keypair = payer;
      if (flags["mint-authority"] !== undefined) {
        mintAuthority = await getKeypairFromFile(flags["mint-authority"]);
      }

      const txId = await mintTo(
        rpc(),
        payer,
        mintPublicKey,
        toPublicKey,
        mintAuthority,
        amount,
      );
      loader.stop(false);
      console.log(
        "\x1b[1mMint tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("mint-to successful");
    } catch (error) {
      this.error(`Failed to create-mint!\n${error}`);
    }
  }
}

export default MintToCommand;
