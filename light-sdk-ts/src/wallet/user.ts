import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { Keypair } from "../keypair";
import { Utxo } from "utxo";
import { Transaction, TransactionParameters } from "transaction";

// TODO: need a wallet
export class User {
  payer: SolanaKeypair;
  seed: string;
  shieldedKeypair: Keypair;
  // TODO: Utxos should be assigned to a merkle tree
  utxos?: Utxo[];
  // TODO: evaluate optimization to store keypairs separately or store utxos in a map<Keypair, Utxo> to not store Keypairs repeatedly

  constructor(poseidon: any, seed: string, solanaKeypair: SolanaKeypair) {
    this.seed = seed;
    this.shieldedKeypair = new Keypair({ poseidon, seed });
    this.payer = solanaKeypair;
  }

  // Fetch utxos should probably be a function such the user object is not occupied while fetching
  // but it would probably be more logical to fetch utxos here as well
  addUtxos() {}
  
  // TODO: evaluate where do we create outUtxos?
  selectUtxos(amount) {}

  // TODO: evaluate whether to move prepareUtxos here
  // maybe it makes sense since I might need new keypairs etc in this process
  // maybe not because we want to keep this class lean

  shield(): TransactionParameters {}

  unshield(): TransactionParameters {}

  transfer(): TransactionParameters {}

  appInteraction() {}


  /*
    *
    *return {
        inputUtxos,
        outputUtxos,
        txConfig: { in: number; out: number },
        verifier, can be verifier object
    }
    * 
    */

  // might be a wrapper for a wallet or dapp to init a user with a wallets sign method
  static initWithSignature() {
    // fetchUtxos
    // return new User();
  }
}
