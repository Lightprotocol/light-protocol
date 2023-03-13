import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeProgram } from "../idls/index";
import { MerkleTree } from "merkleTree/merkleTree";
import { QueuedLeavesPda } from "merkleTree/solMerkleTree";

import { merkleTreeProgramId } from "../constants";
import { Account } from "../account";
import { assert } from "chai";

/**deprecated */
export async function getUnspentUtxo(
  leavesPdas: { account: QueuedLeavesPda }[],
  provider: anchor.Provider,
  account: Account,
  POSEIDON: any,
  merkleTreeProgram: anchor.Program<MerkleTreeProgram>,
  merkleTree: MerkleTree,
  index: number,
) {
  let decryptedUtxos = [];
  for (var i = 0; i < leavesPdas.length; i++) {
    try {
      // decrypt first leaves account and build utxo

      var decryptedUtxo1 = Utxo.decrypt({
        poseidon: POSEIDON,
        encBytes: new Uint8Array(
          Array.from(leavesPdas[i].account.encryptedUtxos),
        ),
        account: account,
        index: leavesPdas[i].account.leftLeafIndex.toNumber(),
      });
      if (!decryptedUtxo1) {
        continue;
      }

      const mtIndex = merkleTree.indexOf(
        decryptedUtxo1?.getCommitment()?.toString(),
      );
      assert.equal(mtIndex.toString(), decryptedUtxo1.index!.toString());

      let nullifier = decryptedUtxo1.getNullifier();
      if (!nullifier) throw new Error("getNullifier of decryptedUtxo failed");
      let nullifierPubkey = (
        await PublicKey.findProgramAddress(
          [
            new anchor.BN(nullifier.toString()).toBuffer(),
            anchor.utils.bytes.utf8.encode("nf"),
          ],
          merkleTreeProgram.programId,
        )
      )[0];
      let accountInfo = await provider.connection.getAccountInfo(
        nullifierPubkey,
      );
      if (
        accountInfo == null &&
        decryptedUtxo1.amounts[1].toString() != "0" &&
        decryptedUtxo1.amounts[0].toString() != "0" &&
        mtIndex.toString() != "-1"
      ) {
        console.log("found unspent leaf");

        decryptedUtxos.push(decryptedUtxo1);
      } else if (i == leavesPdas.length - 1) {
        throw "no unspent leaf found";
      }
    } catch (error) {
      console.log(error);
    }
  }

  return decryptedUtxos[index];
}

//TODO: getSpentUtxos - wrapper over same thing.
/**
 * @params d
 * returns a list of all unspent utxos
 */
export async function getUnspentUtxos({
  leavesPdas,
  provider,
  account,
  poseidon,
  merkleTreeProgram: MerkleTreeProgram,
  merkleTree,
}: {
  leavesPdas: any;
  provider: anchor.Provider;
  account: Account;
  poseidon: any;
  merkleTreeProgram: any;
  merkleTree: MerkleTree;
}): Promise<Utxo[]> {
  let decryptedUtxos: Utxo[] = [];
  // TODO: check performance vs a proper async map and check against fetching nullifiers separately (indexed)
  // TODO: categorize "pending" utxos sent by others
  for (let i = 0; i < leavesPdas.length; i++) {
    const leafPda = leavesPdas[i];
    let decrypted = [
      Utxo.decrypt({
        poseidon: poseidon,
        encBytes: new Uint8Array(
          Array.from(leafPda.account.encryptedUtxos.slice(0, 95)),
        ),
        account,
        index: leavesPdas[i].account.leftLeafIndex.toNumber(),
      }),
      Utxo.decrypt({
        poseidon: poseidon,
        encBytes: new Uint8Array(
          Array.from(leafPda.account.encryptedUtxos.slice(95)),
        ),
        account,
        index: leavesPdas[i].account.leftLeafIndex.toNumber() + 1,
      }),
    ];

    for (let decryptedUtxo of decrypted) {
      if (!decryptedUtxo) continue;

      /** must add index */
      const mtIndex = merkleTree.indexOf(
        decryptedUtxo?.getCommitment()!.toString(),
      );
      // decryptedUtxo.index = mtIndex;
      assert.equal(mtIndex.toString(), decryptedUtxo.index!.toString());

      let nullifier = decryptedUtxo.getNullifier();
      if (!nullifier) continue;

      let nullifierPubkey = PublicKey.findProgramAddressSync(
        [
          Buffer.from(new anchor.BN(nullifier.toString()).toArray()),
          anchor.utils.bytes.utf8.encode("nf"),
        ],
        merkleTreeProgramId, // Merkle...
      )[0];
      let accountInfo = await provider.connection.getAccountInfo(
        nullifierPubkey,
      );
      console.log(
        "inserted -- spent?",
        accountInfo ? "yes" : "no ",
        nullifierPubkey.toBase58(),
        "amount:",
        decryptedUtxo.amounts[0].toNumber(),
      );
      if (
        !accountInfo &&
        (decryptedUtxo.amounts[1].toString() !== "0" ||
          decryptedUtxo.amounts[0].toString() !== "0")
      ) {
        decryptedUtxos.push(decryptedUtxo);
      }
    }
  }
  if (decryptedUtxos.length == 0) {
    console.log("no unspent leaf found");
  }

  return decryptedUtxos;
}
