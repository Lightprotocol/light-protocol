import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { MerkleTreeProgram } from "../idls/index";
import { MerkleTree } from "merkleTree/merkleTree";
import { QueuedLeavesPda } from "merkleTree/solMerkleTree";

import { merkleTreeProgramId } from "../constants";
import { Account } from "../account";

const fetchAccountInfo = async (nullifier: string, connection: Connection) => {
  const nullifierPubkey = PublicKey.findProgramAddressSync(
    [
      Buffer.from(new anchor.BN(nullifier.toString()).toArray()),
      anchor.utils.bytes.utf8.encode("nf"),
    ],
    merkleTreeProgramId,
  )[0];
  return connection.getAccountInfo(nullifierPubkey, "confirmed");
};

const processDecryptedUtxos = async ({
  decryptedUtxo,
  poseidon,
  checkMerkleTreeIndex = false,
  merkleTree,
  connection,
  decryptedUtxos,
  spentUtxos = [],
}: {
  decryptedUtxo?: Utxo;
  checkMerkleTreeIndex?: boolean;
  merkleTree?: any;
  poseidon: any;
  connection: Connection;
  decryptedUtxos: Utxo[];
  spentUtxos?: Utxo[];
}) => {
  if (!decryptedUtxo) return;
  const nullifier = decryptedUtxo.getNullifier(poseidon);
  if (!nullifier) return;
  const accountInfo = await fetchAccountInfo(nullifier, connection);
  const amountsValid =
    decryptedUtxo.amounts[1].toString() !== "0" ||
    decryptedUtxo.amounts[0].toString() !== "0";

  let mtIndexValid = true;
  if (checkMerkleTreeIndex) {
    const mtIndex = merkleTree.indexOf(
      decryptedUtxo.getCommitment(poseidon)?.toString(),
    );
    mtIndexValid = mtIndex.toString() !== "-1";
  }

  if (!accountInfo && amountsValid && mtIndexValid) {
    decryptedUtxos.push(decryptedUtxo);
  } else if (accountInfo && amountsValid) {
    spentUtxos.push(decryptedUtxo);
  }
};

/**
 * @description Retrieves the unspent UTXO at a specified index for a given account.
 * @param leavesPdas An array of QueuedLeavesPda objects containing the encrypted UTXOs.
 * @param provider An instance of the anchor.Provider to interact with the Solana network.
 * @param account The account for which to retrieve the unspent UTXO.
 * @param poseidon A Poseidon hash function instance.
 * @param merkleTree A MerkleTree instance for validating the UTXO.
 * @param index The index at which to retrieve the unspent UTXO.
 * @returns The unspent UTXO at the specified index if it exists, otherwise throws an error.
 */
export async function getUnspentUtxo(
  leavesPdas: { account: QueuedLeavesPda }[],
  provider: anchor.Provider,
  account: Account,
  poseidon: any,
  merkleTree: MerkleTree,
  index: number,
) {
  let decryptedUtxos: Utxo[] = [];

  const tasks = leavesPdas.map((leafPda) => {
    const decryptedUtxo = Utxo.decrypt({
      poseidon: poseidon,
      encBytes: new Uint8Array(Array.from(leafPda.account.encryptedUtxos)),
      account: account,
      index: leafPda.account.leftLeafIndex.toNumber(),
    });

    return processDecryptedUtxos({
      decryptedUtxo: decryptedUtxo!,
      poseidon,
      connection: provider.connection,
      merkleTree,
      checkMerkleTreeIndex: true,
      decryptedUtxos,
    });
  });

  await Promise.all(tasks);

  if (decryptedUtxos.length > index) {
    return decryptedUtxos[index];
  }
  throw "no unspent leaf found";
}

/**
 *  Fetches the decrypted and spent UTXOs for an account from the provided leavesPDAs.
 * @param {Array} leavesPdas - An array of leaf PDAs containing the UTXO data.
 * @param {anchor.Provider} provider - The Anchor provider to interact with the Solana network.
 * @param {Account} account - The user account for which to fetch the UTXOs.
 * @param {Object} poseidon - The Poseidon object used for cryptographic operations.
 * @returns {Promise<{ decryptedUtxos: Utxo[], spentUtxos: Utxo[] }>} A Promise that resolves to an object containing two arrays:
 * - decryptedUtxos: The decrypted UTXOs that have not been spent.
 * - spentUtxos: The decrypted UTXOs that have been spent.
 */
export async function getAccountUtxos({
  leavesPdas,
  provider,
  account,
  poseidon,
}: {
  leavesPdas: any;
  provider: anchor.Provider;
  account: Account;
  poseidon: any;
}): Promise<{ decryptedUtxos: Utxo[]; spentUtxos: Utxo[] }> {
  let decryptedUtxos: Utxo[] = [];
  let spentUtxos: Utxo[] = [];

  const tasks = leavesPdas.flatMap((leafPda: any) => {
    const decrypted = [
      Utxo.decrypt({
        poseidon: poseidon,
        encBytes: new Uint8Array(
          Array.from(leafPda.account.encryptedUtxos.slice(0, 95)),
        ),
        account,
        index: leafPda.account.leftLeafIndex.toNumber(),
      }),
      Utxo.decrypt({
        poseidon: poseidon,
        encBytes: new Uint8Array(
          Array.from(leafPda.account.encryptedUtxos.slice(95)),
        ),
        account,
        index: leafPda.account.leftLeafIndex.toNumber() + 1,
      }),
    ];

    return decrypted.map((decryptedUtxo) =>
      processDecryptedUtxos({
        decryptedUtxo: decryptedUtxo!,
        poseidon,
        connection: provider.connection,
        decryptedUtxos,
        spentUtxos,
      }),
    );
  });

  await Promise.all(tasks);

  return { decryptedUtxos, spentUtxos };
}
