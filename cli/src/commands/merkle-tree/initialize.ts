import { Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";

class InitializeCommand extends Command {
  static description = "Initialize the Merkle Tree.";

  static examples = ["light initialize -p <pubKey>"];

  static flags = {
    pubKey: Flags.string({
      char: "p",
      description: "Solana public key of the Merkle Tree Authority.",
      required: true,
    }),
    message: Flags.boolean({
      char: "m",
      description: "Initialize a new message Merkle Tree.",
      default: false,
      exclusive: ["transaction"],
    }),
    transaction: Flags.boolean({
      char: "t",
      description: "Initialize a new transaction Merkle Tree.",
      default: false,
      exclusive: ["message"],
    }),
  };
  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const { flags } = await this.parse(InitializeCommand);
    const { pubKey, message, transaction } = flags;

    const merkleTreeKey = new PublicKey(pubKey);

    const loader = new CustomLoader(`Initializing new Merkle Tree Account...`);
    loader.start();

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
            this.log("\nInitialized a new transaction Merkle Tree");
          } else if (message) {
            await merkleTreeConfig.initializeNewEventMerkleTree();
            this.log("\nInitialized a new message Merkle Tree");
          }
          this.log("\nMerkle Tree Account initialized successfully");
          this.log(`\nMerkle Tree PubKey: ${merkleTreeKey}\n`);
        } catch (error) {
          this.error(`\nFailed to initialize Merkle Tree Account: ${error}`);
        }
      } else {
        this.log("\nMerkle Tree Account already exists");
      }
      loader.stop();
    } catch (error) {
      loader.stop();

      this.error(`\nFailed to initialize Merkle Tree Account: ${error}`);
    }
  }
}

InitializeCommand.strict = false;

export default InitializeCommand;
