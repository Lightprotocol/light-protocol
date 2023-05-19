import { Command, Flags } from "@oclif/core";
import * as fs from "fs";
import {
  CustomLoader,
  isValidBase58SecretKey,
  isValidURL,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";

class ConfigCommand extends Command {
  static description = "Update the configuration values";
  static examples = [
    "$ light psp-config --rpcUrl https://solana-api.example.com",
    "$ light psp-config --relayerUrl https://relayer.example.com",
    "$ light psp-config --secretKey your <SOLANA_SECRET_KEY>",
    "$ light psp-config --relayerRecipient <RECIPIENT_ADDRESS>",
    "$ light psp-config --lookupTable <LOOKUP_TABLE>",
  ];

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
      description: "solana keypair secretkey in base58 string format",
    }),
    relayerRecipient: Flags.string({
      char: "u",
      description: "Relayer recipient",
    }),
    lookupTable: Flags.string({
      char: "t",
      description: "Look-up table",
    }),
  };

  async run() {
    const { flags } = await this.parse(ConfigCommand);
    const { rpcUrl, relayerUrl, secretKey, relayerRecipient, lookupTable } = flags;

    const loader = new CustomLoader("Updating configuration...");
    loader.start();

    try {
      const config = JSON.parse(fs.readFileSync("config.json", "utf-8"));
      if (rpcUrl) {
        if (isValidURL(rpcUrl)) {
          config.rpcUrl = rpcUrl;
        } else {
          this.error(`\nInvalid URL format`);
        }
      }
      if (relayerUrl) {
        if (isValidURL(relayerUrl)) {
          config.relayerUrl = relayerUrl;
        } else {
          this.error(`\nInvalid URL format`);
        }
      }
      if (secretKey) {
        if (isValidBase58SecretKey(secretKey)) {
          config.secretKey = secretKey;
        } else {
          this.error(`\nInvalid solana keypair base58 string format`);
        }
      }
      if (relayerRecipient) {
        if (new PublicKey(relayerRecipient)) {
          config.relayerRecipient = relayerRecipient;
        } else {
          this.error(`\nInvalid publickey format`);
        }
      }
      if (lookupTable) {
        if (new PublicKey(lookupTable)) {
          config.lookUpTable = lookupTable;
        } else {
          this.error(`\nInvalid publickey format`);
        }
      }
      fs.writeFileSync("config.json", JSON.stringify(config, null, 2));
      this.log("\nConfiguration values updated successfully \x1b[32m✔\x1b[0m");
      loader.stop();
    } catch (err) {
      loader.stop();
      this.error(`\nFailed to update configuration values\n${err}`);
    }
  }
}

export default ConfigCommand;
