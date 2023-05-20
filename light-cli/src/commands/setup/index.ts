import { Command, Flags } from "@oclif/core";
import { Keypair, PublicKey } from "@solana/web3.js";
import { exec } from "child_process";
import {
  createTestAccounts,
  initLookUpTableFromFile,
  setUpMerkleTree,
  sleep,
} from "@lightprotocol/zk.js";
import {
  setRelayerRecipient,
  setAnchorProvider,
  setLookUpTable,
  CustomLoader,
} from "../../utils/utils";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const loader = new CustomLoader("Performing setup tasks...");

    loader.start();

    try {
      exec("sh runScript.sh", (error, stdout, stderr) => {
        if (error) {
          console.error("\nFailed to execute runScript.sh:", error);
          return;
        }
        console.log("\nSetup script executed successfully.");
      });

      await sleep(9000);

      const provider = await setAnchorProvider();

      await createTestAccounts(provider.connection);

      const lookupTable = await initLookUpTableFromFile(provider);

      await setLookUpTable(lookupTable.toString());

      await setUpMerkleTree(provider);

      const relayerRecipientSol = Keypair.generate().publicKey;

      setRelayerRecipient(relayerRecipientSol.toString());

      await provider.connection.requestAirdrop(
        relayerRecipientSol,
        2_000_000_000
      );

      this.log("\nSetup tasks completed successfully.");
      loader.stop();
    } catch (error) {
      loader.stop();

      this.error(`\nSetup tasks failed: ${error}`);
    }
  }
}

export default SetupCommand;
