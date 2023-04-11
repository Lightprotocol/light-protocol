import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Connection } from "@solana/web3.js";
import { Account } from "../account";
import { fetchNullifierAccountInfo } from "../utils";

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
  const accountInfo = await fetchNullifierAccountInfo(nullifier, connection);
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
