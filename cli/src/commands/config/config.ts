import { Command, Flags, ux } from "@oclif/core";
import * as fs from "fs";
import {
  CustomLoader,
  getConfig,
  getRelayerUrl,
  isValidBase58SecretKey,
  isValidURL,
  readWalletFromFile,
  setConfig,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { CONFIG_FILE_NAME, CONFIG_PATH } from "../../psp-utils";
import { Relayer } from "@lightprotocol/zk.js";

class ConfigCommand extends Command {
  static description =
    "Initialize or update the configuration values. The default config path is ~/.config/light/config.json you can set up a custom path with an environment variable export LIGHT_PROTOCOL_CONFIG=path/to/config.json";
  static examples = [
    "$ light config --rpcUrl https://solana-api.example.com",
    "$ light config --relayerUrl https://relayer.example.com",
    "$ light config --secretKey your <SOLANA_SECRET_KEY>",
    "$ light config --lookUpTable <LOOKUP_TABLE>",
    "$ light config --relayerRecipient <RECIPIENT_ADDRESS>",
    "$ light config --relayerPublicKey <RELAYER_PUBLIC_KEY>",
  ];

  static flags = {
    rpcUrl: Flags.string({
      char: "r",
      description: "Solana rpc url.",
    }),
    relayerUrl: Flags.string({
      char: "l",
      description: "Relayer url.",
    }),
    secretKey: Flags.string({
      char: "s",
      description: "Solana keypair secretkey in base58 string format.",
    }),
    relayerRecipient: Flags.string({
      char: "u",
      description: "Relayer recipient",
    }),
    lookUpTable: Flags.string({
      char: "t",
      description: "Lookup Table for versioned transactions.",
    }),
    relayerPublicKey: Flags.string({
      alias: "rp",
      description: "Relayer public key.",
    }),
    syncRelayer: Flags.boolean({
      description: "Syncs the relayer and updates it's public keys.",
      required: false,
    }),
    get: Flags.boolean({
      char: "g",
      description: "Gets the current config values.",
      required: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(ConfigCommand);
    const {
      rpcUrl,
      relayerUrl,
      secretKey,
      relayerRecipient,
      relayerPublicKey,
      get,
      lookUpTable,
      syncRelayer,
    } = flags;

    try {
      const config = getConfig();
      if (get) {
        logConfig(config);
        return;
      }
      const loader = new CustomLoader("Updating configuration...");
      loader.start();
      // TODO: refactor this into accepting default values like localhost, test-relayer, testnet, devnet, mainnet in addition to raw urls
      // http://127.0.0.1:8899
      if (rpcUrl) {
        if (isValidURL(rpcUrl)) {
          config.rpcUrl = rpcUrl;
        } else {
          this.error(`\nInvalid URL format`);
        }
      }
      // http://localhost:3332
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

      if (syncRelayer) {
        let fetchedRelayer = await Relayer.initFromUrl(getRelayerUrl());
        config.relayerPublicKey =
          fetchedRelayer.accounts.relayerPubkey.toBase58();
        config.relayerRecipient =
          fetchedRelayer.accounts.relayerRecipientSol.toBase58();
        config.relayerFee = fetchedRelayer.relayerFee.toString();
        config.highRelayerFee = fetchedRelayer.highRelayerFee.toString();
      }
      // TODO: remove this from config and fetch this from the relayer, use the signer as relayer recipient when using a test relayer
      if (relayerRecipient) {
        if (new PublicKey(relayerRecipient)) {
          config.relayerRecipient = relayerRecipient;
        } else {
          this.error(`\nInvalid publickey format`);
        }
      }
      if (relayerPublicKey) {
        if (new PublicKey(relayerPublicKey)) {
          config.relayerPublicKey = relayerPublicKey;
        } else {
          this.error(`\nInvalid publickey format`);
        }
      }

      if (lookUpTable) {
        if (new PublicKey(lookUpTable)) {
          config.lookUpTable = lookUpTable;
        } else {
          this.error(`\nInvalid public key format`);
        }
      }

      setConfig(config);
      this.log("\nConfiguration values updated successfully \x1b[32m✔\x1b[0m");
      loader.stop(false);
      // logging updated config values
      logConfig(config);
    } catch (error) {
      this.error(`\nFailed to update configuration values\n${error}`);
    }
  }
}

function logConfig(config: any) {
  let tableData = [];

  tableData.push({
    name: "user public key",
    value: readWalletFromFile().publicKey.toBase58(),
  });
  tableData.push({
    name: "rpc url",
    value: config.rpcUrl,
  });

  tableData.push({
    name: "default shield lookup table",
    value: config.lookUpTable,
  });

  tableData.push({
    name: "relayer public key",
    value: config.relayerPublicKey,
  });

  tableData.push({
    name: "relayer url",
    value: config.relayerUrl,
  });
  tableData.push({
    name: "relayer recipient",
    value: config.relayerRecipient,
  });

  // space
  tableData.push({
    name: "",
    value: "",
  });

  ux.table(tableData, {
    name: { header: "" },
    value: { header: "" },
  });
}
export default ConfigCommand;
