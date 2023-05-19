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

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples = [
    "$ light config --rpcUrl https://solana-api.example.com",
    "$ light config --relayerUrl https://relayer.example.com",
    "$ light config --secretKey your solana-keypair-secretkey-in-base58-string-format",
    "$ light config --relayerRecipient <recipient_address>",
    "$ light config --lookupTable <lookup_table>",
  ];

  async run() {
    const { flags } = await this.parse(ConfigCommand);

    const { rpcUrl, relayerUrl, secretKey, relayerRecipient, lookupTable } =
      flags;

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
      this.log("\nConfiguration values updated successfully");
      loader.stop();
    } catch (err) {
      loader.stop();

      this.error(`\nFailed to update configuration values: ${err}`);
    }
  }
}

export default ConfigCommand;
