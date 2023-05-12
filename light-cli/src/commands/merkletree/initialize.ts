import { Command, Flags } from "@oclif/core";
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import {
  getLocalProvider,
  getWalletConfig,
  readPayerFromIdJson,
} from "../../utils";

class InitializeCommand extends Command {
  static description = "Initialize the Merkle Tree Authority";

  static examples = ["light-cli initialize -p <pubKey>"];

  static flags = {
    pubKey: Flags.string({
      char: "p",
      description: "Public key of the Merkle Tree Authority",
      required: true,
    }),
  };

  async run() {
    const { flags } = await this.parse(InitializeCommand);
    const { pubKey } = flags;

    const MERKLE_TREE_KEY = new PublicKey(pubKey);

    try {
      const payer = new anchor.Wallet(readPayerFromIdJson());
      const provider = await getLocalProvider(payer);

      const merkleTreeAccountInfo = await provider.connection.getAccountInfo(
        MERKLE_TREE_KEY
      );
      if (!merkleTreeAccountInfo) {
        let merkleTreeConfig = await getWalletConfig(
          provider,
          MERKLE_TREE_KEY,
          readPayerFromIdJson()
        );
        this.log("Initializing new Merkle Tree Account", "info");
        try {
          const ix = await merkleTreeConfig.initializeNewMerkleTree();
          this.log("Merkle Tree Account initialized successfully");
          this.log(`Merkle Tree PubKey: ${MERKLE_TREE_KEY}\n`);
        } catch (error) {
          this.error(error.message);
        }
      } else {
        this.log("Merkle Tree Account already exists", "info");
      }
    } catch (error) {
      let errorMessage = "Aborted.";
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      this.error(errorMessage);
    }
  }
}

InitializeCommand.strict = false;


export default InitializeCommand;
