import { Command } from "@oclif/core";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { exec } from "child_process";
import {
  createTestAccounts,
  initLookUpTableFromFile,
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
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();

    try {
      exec("sh runScript.sh", (error, stdout, stderr) => {
        if (error) {
          this.error(`Failed to execute runScript.sh: ${error}`)
        }
        this.log("\nSetup script executed successfully \x1b[32m✔\x1b[0m");
      });

      await sleep(9000);

      const anchorProvider = await setAnchorProvider();

      await createTestAccounts(anchorProvider.connection);

      const lookupTable = await initLookUpTableFromFile(anchorProvider);

      setLookUpTable(lookupTable.toString());
      
      const relayerRecipientSol = SolanaKeypair.generate().publicKey;

      setRelayerRecipient(relayerRecipientSol.toString());

      await anchorProvider.connection.requestAirdrop(
        relayerRecipientSol,
        2_000_000_000
      ); 
      
      this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
      loader.stop(false);
    } catch (error) {
      this.error(`\nSetup tasks failed: ${error}`);
    }
  }
}

export default SetupCommand;
