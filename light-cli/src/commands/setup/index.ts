import { Command, Flags } from "@oclif/core";
import { Keypair, PublicKey } from "@solana/web3.js";
import { exec } from "child_process";
import {
  createTestAccounts,
  initLookUpTableFromFile,
  setUpMerkleTree,
  sleep,
} from "light-sdk";
import {
  setRelayerRecipient,
  setAnchorProvider,
  setLookUpTable,
} from "../../utils";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  async run() {
    try {
      exec("sh runScript.sh", (error, stdout, stderr) => {
        if (error) {
          console.error("Failed to execute runScript.sh:", error);
          return;
        }
        console.log("Setup completed successfully.");
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

      this.log("Setup completed successfully.");
    } catch (error) {
      this.error(`Setup failed: ${error}`);
    }
  }
}

export default SetupCommand;
