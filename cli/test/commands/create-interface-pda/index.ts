import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";

import { PublicKey } from "@solana/web3.js";
import { createSplInterface } from "@lightprotocol/compressed-token";

class CreateInterfacePdaCommand extends Command {
  static summary = "Create an SPL interface PDA for an existing mint";

  static examples = ["$ light create-interface-pda --mint <value>"];

  static flags = {
    mint: Flags.string({
      description: "Provide a base58 encoded mint address to register",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(CreateInterfacePdaCommand);

    const loader = new CustomLoader(`Creating SPL interface PDA...\n`);
    loader.start();
    try {
      const payer = defaultSolanaWalletKeypair();
      const mintAddress = new PublicKey(flags.mint);
      const txId = await createSplInterface(rpc(), payer, mintAddress);
      loader.stop(false);
      console.log("\x1b[1mMint public key:\x1b[0m ", mintAddress.toBase58());
      console.log(
        "\x1b[1mMint tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("create-interface-pda successful");
    } catch (error) {
      this.error(`Failed to create-interface-pda!\n${error}`);
    }
  }
}

export default CreateInterfacePdaCommand;
