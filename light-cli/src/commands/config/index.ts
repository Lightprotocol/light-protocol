import { Command, Flags } from "@oclif/core";
import * as fs from "fs";

class ConfigCommand extends Command {
  static description = "Update the configuration values";

  static flags = {
    rpcUrl: Flags.string({
      description: "Solana RPC URL",
    }),
    relayerUrl: Flags.string({
      description: "Relayer URL",
    }),
    secretKey: Flags.string({
      description: "Secret key in string format",
    }),
    relayerRecipient: Flags.string({
      description: "Relayer recipient",
    }),
    lookupTable: Flags.string({
      description: "Look-up table",
    }),
    payer: Flags.string({
      description: "Payer secret key",
    }),
  };

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

      this.log("Configuration values updated successfully");
    } catch (err) {
      this.error(`Failed to update configuration values: ${err}`);
    }
  }
}

export default ConfigCommand;
