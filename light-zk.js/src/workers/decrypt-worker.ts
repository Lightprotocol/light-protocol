//@ts-check
import { Utxo } from "../utxo";
import { fetchNullifierAccountInfo } from "../utils";
import { UtxoError } from "errors";
import { MerkleTreeConfig } from "merkleTree";
import {
  Account,
  DecryptionResult,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  ParsedIndexedTransaction,
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

async function decryptUtxo({
  account,
  encBytes,
  index,
  commitment,
  poseidon,
  merkleTreePdaPublicKey,
  aes,
  verifierProgramLookupTable,
  assetLookupTable,
}: {
  encBytes: Uint8Array;
  index: number;
  commitment: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  poseidon: any;
  aes: boolean;
  verifierProgramLookupTable: string[];
  assetLookupTable: string[];
}): Promise<Utxo | null> {
  let decryptedUtxo = await Utxo.decrypt({
    poseidon,
    encBytes: encBytes,
    account: account,
    index: index,
    commitment,
    aes,
    merkleTreePdaPublicKey,
    verifierProgramLookupTable,
    assetLookupTable,
  });

  // null if utxo did not decrypt -> return nothing and continue
  if (!decryptedUtxo) return null;

  const nullifier = decryptedUtxo.getNullifier(poseidon);
  if (!nullifier) return null;

  const amountsValid =
    decryptedUtxo.amounts[1].toString() !== "0" ||
    decryptedUtxo.amounts[0].toString() !== "0";

  // valid amounts and is not app utxo
  if (
    amountsValid &&
    decryptedUtxo.verifierAddress.toBase58() === new PublicKey(0).toBase58() &&
    decryptedUtxo.appDataHash.toString() === "0"
  ) {
    return decryptedUtxo;
  }

  return null;
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
    merkleTreePdaPublicKey: string,
    aes: boolean,
    verifierProgramLookupTable: string[],
    assetLookupTable: string[],
  ) {
    // Prevent race condition
    await initPromise;
    let account = Account.fromJSON(accountState, poseidon, eddsa);

    let promises: Promise<any>[] = [];

    for (const trx of indexedTransactions) {
      let leftLeafIndex = new BN(trx.firstLeafIndex).toNumber();

      for (let index = 0; index < trx.leaves.length; index += 2) {
        const leafLeft = trx.leaves[index];
        const leafRight = trx.leaves[index + 1];

        promises.push(
          decryptUtxo({
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

            merkleTreePdaPublicKey: new PublicKey(merkleTreePdaPublicKey),
            aes,
            verifierProgramLookupTable: verifierProgramLookupTable,
            assetLookupTable: assetLookupTable,
          }).then((utxo) => ({
            // We need to access leftLeaf when modifying the balance in the mainThread
            // TODO: Instead, we could pass leafLeft as param and resolve directly to it.
            utxo,
            leftLeaf: Uint8Array.from([...leafLeft]),
          })),
          decryptUtxo({
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
            merkleTreePdaPublicKey: new PublicKey(merkleTreePdaPublicKey),
            aes,
            verifierProgramLookupTable: verifierProgramLookupTable,
            assetLookupTable: assetLookupTable,
          }).then((utxo) => ({
            utxo,
            leftLeaf: Uint8Array.from([...leafLeft]),
          })),
        );
      }
    }
    const decryptionResults: DecryptionResult[] = (
      await Promise.all(promises)
    ).filter((obj) => obj.utxo !== null);
    return decryptionResults;
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
