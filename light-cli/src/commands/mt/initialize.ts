import { Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import { getLoader, getWalletConfig, setAnchorProvider } from "../../utils";

class InitializeCommand extends Command {
  static description = "Initialize the Merkle Tree Authority";

  static examples = ["light initialize -p <pubKey>"];

  static flags = {
    pubKey: Flags.string({
      char: "p",
      description: "Public key of the Merkle Tree Authority",
      required: true,
    }),
    message: Flags.boolean({
      char: "m",
      description: "initialize new message merkleTree",
      default: false,
      exclusive: ["transaction"],
    }),
    transaction: Flags.boolean({
      char: "t",
      description: "initialize new transaction merkleTree",
      default: false,
      exclusive: ["message"],
    }),
  };

  async run() {
    const { flags } = await this.parse(InitializeCommand);
    const { pubKey, message, transaction } = flags;

    const MERKLE_TREE_KEY = new PublicKey(pubKey);

    const { loader, end } = getLoader(
      `Initializing new Merkle Tree Account...`
    );

    try {
      const { connection } = await setAnchorProvider();

      const merkleTreeAccountInfo = await connection.getAccountInfo(
        MERKLE_TREE_KEY
      );

      if (!merkleTreeAccountInfo) {
        let merkleTreeConfig = await getWalletConfig(connection);
        try {
          if (transaction) {
            console.log("here 2");
            merkleTreeConfig.initializeNewTransactionMerkleTree();
          } else if (message) {
            console.log("here");
            await merkleTreeConfig.initializeNewMessageMerkleTree();
          }
          this.log("Merkle Tree Account initialized successfully");
          this.log(`Merkle Tree PubKey: ${MERKLE_TREE_KEY}\n`);
        } catch (error) {
          this.error(`${error}`);
        }
      } else {
        this.log("Merkle Tree Account already exists");
      }
      end(loader);
    } catch (error) {
      let errorMessage = "Aborted.";
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      end(loader);
      this.error(errorMessage);
    }
  }
}

InitializeCommand.strict = false;

export default InitializeCommand;
