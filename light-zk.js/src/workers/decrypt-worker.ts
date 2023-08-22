import { Utxo } from "../utxo";
import { fetchNullifierAccountInfo } from "../utils";
import { UtxoError } from "errors";
import { MerkleTreeConfig } from "merkleTree";
import {
  Account,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  ParsedIndexedTransaction,
  decryptAddUtxoToBalance,
} from "index";
import { Connection, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { expose, windowEndpoint } from "comlink/dist/esm/comlink";
import nodeEndpoint from "comlink/dist/esm/node-adapter";
const circomlibjs = require("circomlibjs");
let poseidon: any;
let eddsa: any;
let initPromise: Promise<void>;

function initCircomLib() {
  return new Promise<void>(async (resolve, reject) => {
    try {
      poseidon = await circomlibjs.buildPoseidonOpt();
      eddsa = await circomlibjs.buildEddsa();
      resolve();
    } catch (error) {
      reject(error);
    }
  });
}

// Init on file mount
initPromise = initCircomLib();

const workerMethods = {
  async decryptStorageIndices(
    accountState: string,
    indexedTransactions: ParsedIndexedTransaction[],
    assetLookupTable: string[],
    verifierProgramLookupTable: string[],
    url: string = "http://127.0.0.1:8899",
  ): Promise<{ decryptedStorageUtxos: any; spentUtxos: any }> {
    let connection = new Connection(url, "confirmed");

    // Prevent race condition
    await initPromise;

    var decryptedStorageUtxos: Utxo[] = [];
    var spentUtxos: Utxo[] = [];

    const account = Account.fromJSON(accountState, poseidon, eddsa);

    for (const data of indexedTransactions) {
      let decryptedUtxo = null;
      var index = data.firstLeafIndex.toNumber();
      for (var [, leaf] of data.leaves.entries()) {
        try {
          decryptedUtxo = await Utxo.decrypt({
            poseidon,
            account,
            encBytes: Uint8Array.from(data.message),
            // appDataIdl: idl,
            aes: true,
            index: index,
            commitment: Uint8Array.from(leaf),
            merkleTreePdaPublicKey: MerkleTreeConfig.getEventMerkleTreePda(),
            compressed: false,
            verifierProgramLookupTable,
            assetLookupTable,
          });
          if (decryptedUtxo !== null) {
            const nfExists = await fetchNullifierAccountInfo(
              decryptedUtxo.getNullifier(poseidon)!,
              connection,
            );
            if (!nfExists) {
              decryptedStorageUtxos.push(decryptedUtxo);
            } else {
              spentUtxos.push(decryptedUtxo);
            }
          }
          index++;
        } catch (e) {
          if (!(e instanceof UtxoError) || e.code !== "INVALID_APP_DATA_IDL") {
            throw e;
          }
        }
      }
    }
    return { decryptedStorageUtxos, spentUtxos };
  },

  async decryptUtxosInTransactions(
    indexedTransactions: ParsedIndexedTransaction[],
    accountState: string,
    balance: any,
    merkleTreePdaPublicKey: string,
    aes: boolean,
    verifierProgramLookupTable: string[],
    assetLookupTable: string[],
    url: string = "http://127.0.0.1:8899",
  ) {
    let connection = new Connection(url, "confirmed");

    // Prevent race condition
    await initPromise;

    let account = Account.fromJSON(accountState, poseidon, eddsa);

    for (const trx of indexedTransactions) {
      let leftLeafIndex = new BN(trx.firstLeafIndex).toNumber();

      for (let index = 0; index < trx.leaves.length; index += 2) {
        const leafLeft = trx.leaves[index];
        const leafRight = trx.leaves[index + 1];

        await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.encryptedUtxos.slice(
              0,
              NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
            ),
          ),
          index: leftLeafIndex,
          commitment: Buffer.from([...leafLeft]),
          account,
          poseidon,
          connection,
          balance,
          merkleTreePdaPublicKey: new PublicKey(merkleTreePdaPublicKey),
          leftLeaf: Uint8Array.from([...leafLeft]),
          aes,
          verifierProgramLookupTable: verifierProgramLookupTable,
          assetLookupTable: assetLookupTable,
        });
        await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.encryptedUtxos.slice(
              120,
              120 + NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
            ),
          ),
          index: leftLeafIndex + 1,
          commitment: Buffer.from([...leafRight]),
          account,
          poseidon,
          connection,
          balance,
          merkleTreePdaPublicKey: new PublicKey(merkleTreePdaPublicKey),
          leftLeaf: Uint8Array.from([...leafLeft]),
          aes,
          verifierProgramLookupTable: verifierProgramLookupTable,
          assetLookupTable: assetLookupTable,
        });
      }
    }
    return balance;
  },
};

let nodeEndpointContext;

if (typeof window === "undefined") {
  // Node.js environment
  const { parentPort } = require("worker_threads");
  nodeEndpointContext = nodeEndpoint(parentPort);
} else {
  // Browser environment
  nodeEndpointContext = windowEndpoint(self);
}

expose(workerMethods, nodeEndpointContext);
