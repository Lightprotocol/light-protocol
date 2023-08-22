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
    oldMerkleTreePubkey: Flags.string({
      char: "o",
      description: "Solana public key of the old Merkle Tree.",
      required: true,
    }),
    newMerkleTreePubkey: Flags.string({
      char: "n",
      description: "Solana public key of the new Merkle Tree.",
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
    const {
      oldMerkleTreePubkeyString,
      newMerkleTreePubkeyString,
      message,
      transaction,
    } = flags;

    const oldMerkleTreePubkey = new PublicKey(oldMerkleTreePubkeyString);
    const newMerkleTreePubkey = new PublicKey(newMerkleTreePubkeyString);

    const loader = new CustomLoader(`Initializing new Merkle Tree Account...`);
    loader.start();

    try {
      const { connection } = await setAnchorProvider();

      const newMerkleTreeAccountInfo = await connection.getAccountInfo(
        newMerkleTreePubkey
      );

      if (!newMerkleTreeAccountInfo) {
        let merkleTreeConfig = await getWalletConfig(connection);
        try {
          if (transaction) {
            merkleTreeConfig.initializeNewTransactionMerkleTree(
              oldMerkleTreePubkey,
              newMerkleTreePubkey
            );
            this.log("\nInitialized a new transaction Merkle Tree");
          } else if (message) {
            await merkleTreeConfig.initializeNewEventMerkleTree();
            this.log("\nInitialized a new message Merkle Tree");
          }
          this.log("\nMerkle Tree Account initialized successfully");
          this.log(`\nMerkle Tree PubKey: ${newMerkleTreePubkeyString}\n`);
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
