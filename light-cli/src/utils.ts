import * as fs from "fs";
import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";

import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  Provider,
  Relayer,
} from "light-sdk";

require("dotenv").config();

var getDirName = require("path").dirname;
let provider: Provider;
let relayer: Relayer;

export const createNewWallet = () => {
  const keypair: solana.Keypair = solana.Keypair.generate();
  const secretKey: solana.Ed25519SecretKey = keypair.secretKey;
  try {
    fs.mkdirSync(getDirName("./light-test-cache/secret.txt"));
    fs.writeFileSync(
      "./light-test-cache/secret.txt",
      JSON.stringify(Array.from(secretKey))
    );
    console.log("- secret created and cached");
    return keypair;
  } catch (e: any) {
    throw new Error(`error writing secret.txt: ${e}`);
  }
};

export const getConnection = () =>
  new solana.Connection("http://127.0.0.1:8899");

export const readWalletFromFile = () => {
  let secretKey: Array<number> = [];
  try {
    let data: string = fs.readFileSync("./light-test-cache/secret.txt", "utf8");
    secretKey = JSON.parse(data);

    let asUint8Array: Uint8Array = new Uint8Array(secretKey);
    let keypair: solana.Keypair = solana.Keypair.fromSecretKey(asUint8Array);

    console.log("Wallet found!", keypair.publicKey.toString());
    return keypair;
  } catch (e: any) {
    throw new Error("secret.txt not found or corrupted!");
  }
};

export const setAnchorProvider = async (): Promise<anchor.AnchorProvider> => {

  const configPath = "rpc-config.json";
  const rpcUrl = 

  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = await readRpcEndpointFromFile(configPath); // runscript starts dedicated validator on this port.

  const providerAnchor = anchor.AnchorProvider.local(
    await readRpcEndpointFromFile(configPath),
    confirmConfig
  );

  anchor.setProvider(providerAnchor);

  return providerAnchor;
};

export const getLightProvider = async () => {
  if (!provider) {
    const relayer = await getRelayer();

    provider = await Provider.init({
      wallet: readWalletFromFile(),
      relayer,
    });

    return provider;
  }
  return provider;
};

export const getRelayer = async () => {
  if (!relayer) {

    const wallet = readWalletFromFile()

    relayer = new Relayer(
      wallet.publicKey,
      new solana.PublicKey(process.env.LOOK_UP_TABLE || ""),
      getKeyPairFromEnv("RELAYER_RECIPIENT").publicKey,
      relayerFee
    );

    return relayer;
  }
  return relayer;
};

export function updateRpcEndpoint(rpcEndpoint: string) {
  const configPath = "rpc-config.json"; // Path to the configuration file

  return new Promise<void>((resolve, reject) => {
    fs.readFile(configPath, "utf8", (err, data) => {
      if (err) {
        if (err.code === "ENOENT") {
          // Config file doesn't exist, create a new one
          const config = { rpcEndpoint };

          fs.writeFile(
            configPath,
            JSON.stringify(config, null, 2),
            "utf8",
            (err) => {
              if (err) {
                reject(
                  new Error(
                    `Failed to create the configuration file: ${err.message}`
                  )
                );
                return;
              }

              resolve();
            }
          );
        } else {
          reject(
            new Error(`Failed to read the configuration file: ${err.message}`)
          );
        }
        return;
      }

      try {
        const config = JSON.parse(data);
        config.rpcEndpoint = rpcEndpoint;

        fs.writeFile(
          configPath,
          JSON.stringify(config, null, 2),
          "utf8",
          (err) => {
            if (err) {
              reject(
                new Error(
                  `Failed to update the RPC endpoint in the configuration file: ${err.message}`
                )
              );
              return;
            }

            resolve();
          }
        );
      } catch (err) {
        reject(
          new Error(`Failed to parse the configuration file: ${err.message}`)
        );
      }
    });
  });
}

export function readRpcEndpointFromFile(configPath: string): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    fs.readFile(configPath, 'utf8', (err, data) => {
      if (err) {
        if (err.code === 'ENOENT') {
          reject(new Error(`Configuration file not found at path: ${configPath}`));
        } else {
          reject(new Error(`Failed to read the configuration file: ${err.message}`));
        }
        return;
      }

      try {
        const config = JSON.parse(data);
        const rpcEndpoint = config.rpcEndpoint;

        resolve(rpcEndpoint);
      } catch (err) {
        reject(new Error(`Failed to parse the configuration file: ${err.message}`));
      }
    });
  });
}