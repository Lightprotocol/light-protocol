import { Command, Flags, ux } from "@oclif/core";
import {
  CustomLoader,
  getConfig,
  isValidURL,
  setConfig,
} from "../../utils/utils";

class ConfigCommand extends Command {
  static description =
    "Initialize or update the configuration values. The default config path is ~/.config/light/config.json you can set up a custom path with an environment variable export LIGHT_PROTOCOL_CONFIG=path/to/config.json";
  static examples = [
    "$ light config --solanaRpcUrl https://solana-api.example.com",
  ];

  static flags = {
    solanaRpcUrl: Flags.string({
      description: "Solana RPC url",
    }),
    indexerUrl: Flags.string({
      description: "Indexer url",
    }),
    proverUrl: Flags.string({
      description: "Prover url",
    }),
    get: Flags.boolean({
      description: "Gets the current config values",
      required: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(ConfigCommand);
    const { solanaRpcUrl, indexerUrl, proverUrl, get } = flags;

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
      if (indexerUrl) {
        if (isValidURL(indexerUrl)) {
          config.indexerUrl = indexerUrl;
        } else {
          this.error(`\nInvalid URL format`);
        }
      }
      if (proverUrl) {
        if (isValidURL(proverUrl)) {
          config.proverUrl = proverUrl;
        } else {
          this.error(`\nInvalid URL format`);
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
    name: "Solana RPC URL",
    value: config.solanaRpcUrl,
  });

  tableData.push({
    name: "Indexer URL",
    value: config.indexerUrl,
  });

  tableData.push({
    name: "Prover URL",
    value: config.proverUrl,
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
