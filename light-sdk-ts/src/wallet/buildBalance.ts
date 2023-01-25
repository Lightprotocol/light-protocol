import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { MerkleTreeProgram } from "../idls/index";
import { MerkleTree } from "merkleTree/merkleTree";

export async function getUnspentUtxo(
  leavesPdas,
  provider: anchor.Provider,
<<<<<<< HEAD
  KEYPAIR: Keypair,
  POSEIDON: any,
  merkleTreeProgram: anchor.Program<MerkleTreeProgram>,
  merkleTree: MerkleTree,
  index: number,
=======
  encryptionKeypair,
  KEYPAIR,
  feeAsset,
  mint,
  POSEIDON,
  merkleTreeProgram: MerkleTreeProgram,
>>>>>>> 3bd792a7 (rm nm)
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
  encryptionKeypair,
  keypair,
  feeAsset,
  mint,
  poseidon,
  merkleTreeProgram: MerkleTreeProgram,
}: {
  leavesPdas: any;
  provider: anchor.Provider;
  encryptionKeypair: any;
  keypair: any;
  feeAsset: any;
  mint: any;
  poseidon: any;
  merkleTreeProgram: any;
}): Promise<Utxo[]> {
  let decryptedUtxo1;
  for (var i = 0; i < leavesPdas.length; i++) {
    try {
      // decrypt first leaves account and build utxo


      decryptedUtxo1 = Utxo.decrypt({
        poseidon: poseidon,
        encBytes: new Uint8Array(
          Array.from(leavesPdas[i].account.encryptedUtxos),
        ),
        keypair: keypair,
      });

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
        decryptedUtxo1 &&
        decryptedUtxo1.amounts[1].toString() != "0" &&
        decryptedUtxo1.amounts[0].toString() != "0"
      ) {
        console.log("found unspent leaf");
        return [decryptedUtxo1];
      } else if (i == leavesPdas.length - 1) {
        throw "no unspent leaf found";
      }
    } catch (error) {
      console.log(error);
    }
  }
  return [];
}
