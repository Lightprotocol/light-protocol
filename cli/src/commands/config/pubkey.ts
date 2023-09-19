import { Command } from "@oclif/core";
import { CustomLoader, getPayer } from "../../utils";

class PubkeyCommand extends Command {
  static description =
    "Get the Solana public key from the secret key specified in configuration.";

  static examples = ["light config:pubkey"];

  async run() {
    const loader = new CustomLoader("Retrieving the public key");
    loader.start();

    const payer = getPayer();
    loader.stop(false);
    this.log(payer.publicKey.toBase58());
  }
}

export default PubkeyCommand;
