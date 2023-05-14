import { Command, Flags } from "@oclif/core";
import { Keypair, PublicKey } from "@solana/web3.js";
import { exec } from "child_process";
import { createTestAccounts, setUpMerkleTree, sleep } from "light-sdk";
import { setRelayerRecipient, setAnchorProvider } from "../../utils";

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

      await sleep(7000);

      const provider = await setAnchorProvider();

      await createTestAccounts(provider.connection);

      await setUpMerkleTree(provider);

      const relayerRecipientSol = Keypair.generate().publicKey;

      setRelayerRecipient(relayerRecipientSol.toString());

      await provider.connection.requestAirdrop(
        relayerRecipientSol,
        2_000_000_000
      );

      console.log("Setup completed successfully.");
    } catch (error) {
      console.error("Setup failed:", error);
    }
  }
}

export default SetupCommand;
