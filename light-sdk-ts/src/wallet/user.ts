import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { Keypair } from "../keypair";
import { Utxo } from "utxo";

// TODO: need a wallet 
export class User {
    payer: SolanaKeypair
    seed: string
    shieldedKeypair: Keypair
    utxos?: Utxo[]

    constructor(poseidon: any, seed: string, solanaKeypair: SolanaKeypair) {
        this.seed = seed;
        this.shieldedKeypair = new Keypair({poseidon, seed});
        this.payer = solanaKeypair;
    }


    // Fetch utxos should probably be a function such the user object is not occupied while fetching
    addUtxos() {

    }

    selectUtxo() {

    }


}