import { Command, Flags, ux } from "@oclif/core";
import {
  CustomLoader,
  getConfig,
  getRpcUrl,
  isValidBase58SecretKey,
  isValidURL,
  readWalletFromFile,
  setConfig,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { Rpc } from "@lightprotocol/zk.js";

class ConfigCommand extends Command {
  static description =
    "Initialize or update the configuration values. The default config path is ~/.config/light/config.json you can set up a custom path with an environment variable export LIGHT_PROTOCOL_CONFIG=path/to/config.json";
  static examples = [
    "$ light config --solanaRpcUrl https://solana-api.example.com",
    "$ light config --rpcUrl https://rpc.example.com",
    "$ light config --secretKey your <SOLANA_SECRET_KEY>",
    "$ light config --lookUpTable <LOOKUP_TABLE>",
    "$ light config --rpcRecipient <RECIPIENT_ADDRESS>",
    "$ light config --rpcPublicKey <RPC_PUBLIC_KEY>",
  ];

  static flags = {
    solanaRpcUrl: Flags.string({
      char: "r",
      description: "Solana rpc url.",
    }),
    rpcUrl: Flags.string({
      char: "l",
      description: "Rpc url.",
    }),
    secretKey: Flags.string({
      char: "s",
      description: "Solana keypair secretkey in base58 string format.",
    }),
    rpcRecipient: Flags.string({
      char: "u",
      description: "Rpc recipient",
    }),
    lookUpTable: Flags.string({
      char: "t",
      description: "Lookup Table for versioned transactions.",
    }),
    rpcPublicKey: Flags.string({
      alias: "rp",
      description: "Rpc public key.",
    }),
    syncRpc: Flags.boolean({
      description: "Syncs the rpc and updates it's public keys.",
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
      solanaRpcUrl,
      rpcUrl,
      secretKey,
      rpcRecipient,
      rpcPublicKey,
      get,
      lookUpTable,
      syncRpc,
    } = flags;

    try {
      const config = getConfig();
      if (get) {
        logConfig(config);
        return;
      }
      const loader = new CustomLoader("Updating configuration...");
      loader.start();
      // TODO: refactor this into accepting default values like localhost, test-rpc, testnet, devnet, mainnet in addition to raw urls
      // http://127.0.0.1:8899
      if (solanaRpcUrl) {
        if (isValidURL(solanaRpcUrl)) {
          config.solanaRpcUrl = solanaRpcUrl;
        } else {
          this.error(`\nInvalid URL format`);
        }
      }
      // http://localhost:3332
      if (rpcUrl) {
        if (isValidURL(rpcUrl)) {
          config.rpcUrl = rpcUrl;
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

      if (syncRpc) {
        const fetchedRpc = await Rpc.initFromUrl(getRpcUrl());
        config.rpcPublicKey = fetchedRpc.accounts.rpcPubkey.toBase58();
        config.rpcRecipient = fetchedRpc.accounts.rpcRecipientSol.toBase58();
        config.rpcFee = fetchedRpc.rpcFee.toString();
        config.highRpcFee = fetchedRpc.highRpcFee.toString();
      }
      // TODO: remove this from config and fetch this from the rpc, use the signer as rpc recipient when using a test rpc
      if (rpcRecipient) {
        // eslint-disable-next-line
        if (new PublicKey(rpcRecipient)) {
          config.rpcRecipient = rpcRecipient;
        } else {
          this.error(`\nInvalid publickey format`);
        }
      }
      if (rpcPublicKey) {
        // eslint-disable-next-line
        if (new PublicKey(rpcPublicKey)) {
          config.rpcPublicKey = rpcPublicKey;
        } else {
          this.error(`\nInvalid publickey format`);
        }
      }

      if (lookUpTable) {
        // eslint-disable-next-line
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
  const tableData = [];

  tableData.push({
    name: "user public key",
    value: readWalletFromFile().publicKey.toBase58(),
  });
  tableData.push({
    name: "solana rpc url",
    value: config.solanaRpcUrl,
  });

  tableData.push({
    name: "default compress lookup table",
    value: config.lookUpTable,
  });

  tableData.push({
    name: "rpc public key",
    value: config.rpcPublicKey,
  });

  tableData.push({
    name: "rpc url",
    value: config.rpcUrl,
  });
  tableData.push({
    name: "rpc recipient",
    value: config.rpcRecipient,
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
