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

/**deprecated */
export async function getUnspentUtxo(
  leavesPdas: { account: QueuedLeavesPda }[],
  provider: anchor.Provider,
  account: Account,
  poseidon: any,
  merkleTree: MerkleTree,
  index: number,
) {
  let decryptedUtxos: Utxo[] = [];
  const processDecryptedUtxo = async (decryptedUtxo: Utxo | null) => {
    if (!decryptedUtxo) return;

    const mtIndex = merkleTree.indexOf(
      decryptedUtxo.getCommitment(poseidon)?.toString(),
    );
    const nullifier = decryptedUtxo.getNullifier(poseidon);
    if (!nullifier) return;

    const accountInfo = await fetchAccountInfo(nullifier, provider.connection);
    const amountsValid =
      decryptedUtxo.amounts[1].toString() !== "0" &&
      decryptedUtxo.amounts[0].toString() !== "0";
    const mtIndexValid = mtIndex.toString() !== "-1";

    if (accountInfo === null && amountsValid && mtIndexValid) {
      decryptedUtxos.push(decryptedUtxo);
    }
  };

  const tasks = leavesPdas.map((leafPda) => {
    const decryptedUtxo = Utxo.decrypt({
      poseidon: poseidon,
      encBytes: new Uint8Array(Array.from(leafPda.account.encryptedUtxos)),
      account: account,
      index: leafPda.account.leftLeafIndex.toNumber(),
    });

    return processDecryptedUtxo(decryptedUtxo);
  });

  await Promise.all(tasks);

  if (decryptedUtxos.length > index) {
    return decryptedUtxos[index];
  }
  throw "no unspent leaf found";
}

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

  const processDecryptedUtxos = async (decryptedUtxo: Utxo | null) => {
    if (!decryptedUtxo) return;
    const nullifier = decryptedUtxo.getNullifier(poseidon);
    if (!nullifier) return;
    const accountInfo = await fetchAccountInfo(nullifier, provider.connection);
    const amountsValid =
      decryptedUtxo.amounts[1].toString() !== "0" ||
      decryptedUtxo.amounts[0].toString() !== "0";

    console.log(
      "inserted -- spent?",
      accountInfo ? "yes" : "no ",
      "amount:",
      decryptedUtxo.amounts[0].toNumber(),
    );

    if (!accountInfo && amountsValid) {
      decryptedUtxos.push(decryptedUtxo);
    } else if (accountInfo && amountsValid) {
      spentUtxos.push(decryptedUtxo);
    }
  };

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
      processDecryptedUtxos(decryptedUtxo),
    );
  });

  await Promise.all(tasks);

  return { decryptedUtxos, spentUtxos };
}
