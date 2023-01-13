import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { PublicKey} from "@solana/web3.js";

export async function getUnspentUtxo(
  leavesPdas,
  provider: anchor.Provider,
  encryptionKeypair,
  KEYPAIR,
  FEE_ASSET,
  mint,
  POSEIDON,
  merkleTreeProgram: MerkleTreeProgram
) {
  let decryptedUtxo1;
  for (var i = 0; i < leavesPdas.length; i++) {
    try {
      // decrypt first leaves account and build utxo

      decryptedUtxo1 = Utxo.decrypt({
        poseidon: POSEIDON,
        encBytes: new Uint8Array(
          Array.from(leavesPdas[i].account.encryptedUtxos)
        ),
        keypair: KEYPAIR,
      });

      let nullifier = decryptedUtxo1.getNullifier();

      let nullifierPubkey = (
        await PublicKey.findProgramAddress(
          [
            new anchor.BN(nullifier.toString()).toBuffer(),
            anchor.utils.bytes.utf8.encode("nf"),
          ],
          merkleTreeProgram.programId
        )
      )[0];
      let accountInfo = await provider.connection.getAccountInfo(
        nullifierPubkey
      );

      if (
        accountInfo == null &&
        decryptedUtxo1.amounts[1].toString() != "0" &&
        decryptedUtxo1.amounts[0].toString() != "0"
      ) {
        console.log("found unspent leaf");
        return decryptedUtxo1;
      } else if (i == leavesPdas.length - 1) {
        throw "no unspent leaf found";
      }
    } catch (error) {
      console.log(error);
    }
  }
}

