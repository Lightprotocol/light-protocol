import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";

import { PublicKey } from "@solana/web3.js";
import { createTokenPool } from "@lightprotocol/compressed-token";

class RegisterMintCommand extends Command {
  static summary = "Register an existing mint with the CompressedToken program";

  static examples = ["$ light create-token-pool --mint-decimals 5"];

  static flags = {
    mint: Flags.string({
      description: "Provide a base58 encoded mint address to register",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(RegisterMintCommand);

    const loader = new CustomLoader(`Performing create-token-pool...\n`);
    loader.start();
    try {
      const payer = defaultSolanaWalletKeypair();
      const mintAddress = new PublicKey(flags.mint);
      const txId = await createTokenPool(rpc(), payer, mintAddress);
      loader.stop(false);
      console.log("\x1b[1mMint public key:\x1b[0m ", mintAddress.toBase58());
      console.log(
        "\x1b[1mMint tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("create-token-pool successful");
    } catch (error) {
      this.error(`Failed to create-token-pool!\n${error}`);
    }
  }
}

export default RegisterMintCommand;
