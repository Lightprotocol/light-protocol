import { Command, Flags } from "@oclif/core";
import * as fs from "fs";
import { CustomLoader } from "../../utils/utils";

class ConfigCommand extends Command {
  static description = "Update the configuration values";

  static flags = {
    rpcUrl: Flags.string({
      char: "r",
      description: "Solana RPC URL",
    }),
    relayerUrl: Flags.string({
      char: "l",
      description: "Relayer URL",
    }),
    secretKey: Flags.string({
      char: "s",
      description: "Secret key in string format",
    }),
    relayerRecipient: Flags.string({
      char: "u",
      description: "Relayer recipient",
    }),
    lookupTable: Flags.string({
      char: "t",
      description: "Look-up table",
    }),
    payer: Flags.string({
      char: "p",
      description: "Payer secret key",
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples = [
    "$ light config --rpcUrl https://solana-api.example.com",
    "$ light config --relayerUrl https://relayer.example.com",
    "$ light config --secretKey 0123456789abcdef",
    "$ light config --relayerRecipient <recipient_address>",
    "$ light config --lookupTable <lookup_table>",
    "$ light config --payer <payer_secret_key>",
  ];

  async run() {
    const { flags } = await this.parse(ConfigCommand);

    const {
      rpcUrl,
      relayerUrl,
      secretKey,
      relayerRecipient,
      lookupTable,
      payer,
    } = flags;

    const loader = new CustomLoader("Updating configuration...");
    loader.start();

    try {
      const config = JSON.parse(fs.readFileSync("config.json", "utf-8"));

      if (rpcUrl) {
        config.rpcUrl = rpcUrl;
      }
      if (relayerUrl) {
        config.relayerUrl = relayerUrl;
      }
      if (secretKey) {
        config.secretKey = secretKey;
      }
      if (relayerRecipient) {
        config.relayerRecipient = relayerRecipient;
      }
      if (lookupTable) {
        config.lookUpTable = lookupTable;
      }
      if (payer) {
        config.payer = payer;
      }

      fs.writeFileSync("config.json", JSON.stringify(config, null, 2));
      this.log("\nConfiguration values updated successfully");
      loader.stop();
    } catch (err) {
      loader.stop();

      this.error(`\nFailed to update configuration values: ${err}`);
    }
  }
}

export default ConfigCommand;
