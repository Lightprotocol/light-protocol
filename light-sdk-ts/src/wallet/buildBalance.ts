import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { MerkleTreeProgram } from "../idls/index";
import { MerkleTree } from "merkleTree/merkleTree";

/**deprecated */
export async function getUnspentUtxo(
  leavesPdas,
  provider: anchor.Provider,
  KEYPAIR: Keypair,
  POSEIDON: any,
  merkleTreeProgram: anchor.Program<MerkleTreeProgramIdl>,
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
        keypair: KEYPAIR,
      });

      const mtIndex = merkleTree.indexOf(
        decryptedUtxo1?.getCommitment()?.toString(),
      );

      decryptedUtxo1?.index = mtIndex;

      let nullifier = decryptedUtxo1.getNullifier();

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

export async function getUnspentUtxos({
  leavesPdas,
  provider,
  keypair,
  poseidon,
  merkleTreeProgram: MerkleTreeProgram,
}: {
  leavesPdas: any;
  provider: anchor.Provider;
  keypair: any;
  poseidon: any;
  merkleTreeProgram: any;
}): Promise<Utxo[]> {
  let decryptedUtxos: Utxo[] = [];
  // TODO: check performance vs a proper async map and check againsdt fetching nullifiers separately (indexed)
  // TODO: categorize "pending" utxos sent by others
  let promises = leavesPdas.map(async (leafPda: any, i: number) => {
    let decryptedUtxo = Utxo.decrypt({
      poseidon: poseidon,
      encBytes: new Uint8Array(Array.from(leafPda.account.encryptedUtxos)),
      keypair: keypair,
    });
    if (!decryptedUtxo) return;
    let nullifier = decryptedUtxo.getNullifier();
    if (!nullifier) return;
    let nullifierPubkey = (
      await PublicKey.findProgramAddress(
        [
          new anchor.BN(nullifier.toString()).toBuffer(),
          anchor.utils.bytes.utf8.encode("nf"),
        ],
        MerkleTreeProgram.programId,
      )
    )[0];
    let accountInfo = await provider.connection.getAccountInfo(nullifierPubkey);
    if (
      accountInfo === null &&
      decryptedUtxo.amounts[1].toString() != "0" &&
      decryptedUtxo.amounts[0].toString() != "0"
    ) {
      console.log("found unspent leaf");
      decryptedUtxos.push(decryptedUtxo);
    } else if (i == leavesPdas.length - 1) {
      throw "no unspent leaf found";
    }
  });
  await Promise.all(promises);
  return decryptedUtxos;
}
