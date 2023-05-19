import { Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import { getLoader, getWalletConfig, setAnchorProvider } from "../../utils";

class InitializeCommand extends Command {
  static description = "Initialize the Merkle Tree Authority";

  static examples = ["light initialize -p <pubKey>"];

  static flags = {
    pubKey: Flags.string({
      char: "p",
      description: "Solana public key of the Merkle Tree Authority",
      required: true,
    }),
    message: Flags.boolean({
      char: "m",
      description: "Initialize a new message Merkle Tree",
      default: false,
      exclusive: ["transaction"],
    }),
    transaction: Flags.boolean({
      char: "t",
      description: "Initialize a new transaction Merkle Tree",
      default: false,
      exclusive: ["message"],
    }),
  };

  async run() {
    const { flags } = await this.parse(InitializeCommand);
    const { pubKey, message, transaction } = flags;

    const merkleTreeKey = new PublicKey(pubKey);

    const { loader, end } = getLoader(
      `Initializing new Merkle Tree Account...`
    );

    try {
      const { connection } = await setAnchorProvider();

      const merkleTreeAccountInfo = await connection.getAccountInfo(
        merkleTreeKey
      );

      if (!merkleTreeAccountInfo) {
        let merkleTreeConfig = await getWalletConfig(connection);
        try {
          if (transaction) {
            merkleTreeConfig.initializeNewTransactionMerkleTree();
            this.log("Initialized a new transaction Merkle Tree");
          } else if (message) {
            await merkleTreeConfig.initializeNewMessageMerkleTree();
            this.log("Initialized a new message Merkle Tree");
          }
          this.log("Merkle Tree Account initialized successfully");
          this.log(`Merkle Tree PubKey: ${merkleTreeKey}\n`);
        } catch (error) {
          this.error(`Failed to initialize Merkle Tree Account: ${error}`);
        }
      } else {
        this.log("Merkle Tree Account already exists");
      }
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Failed to initialize Merkle Tree Account: ${error}`);
    }
  }
}

InitializeCommand.strict = false;

export default InitializeCommand;
