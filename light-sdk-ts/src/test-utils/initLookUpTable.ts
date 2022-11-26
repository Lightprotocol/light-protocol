import { Provider } from "@project-serum/anchor";
import {PublicKey, AddressLookupTableProgram, Keypair, SystemProgram, sendAndConfirmTransaction, Transaction} from "@solana/web3.js"
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {assert } from "chai";
import {toBufferLE} from 'bigint-buffer';
import { PathOrFileDescriptor, readFileSync, writeFile } from "fs";

import {
    MERKLE_TREE_KEY,
    ADMIN_AUTH_KEYPAIR,
    AUTHORITY,
    merkleTreeProgram,
    verifierProgramZero,
    REGISTERED_POOL_PDA_SOL,
    DEFAULT_PROGRAMS,
    TOKEN_AUTHORITY,
    REGISTERED_POOL_PDA_SPL_TOKEN,
    PRE_INSERTED_LEAVES_INDEX,

} from "../constants"

// TODO: create cli function to create a lookup table for apps

// Probably only works for testing
// TODO: extend with custom accounts to add
export async function initLookUpTableFromFile(provider: anchor.Provider,path: PathOrFileDescriptor = `lookUpTable.txt`)/*: Promise<PublicKey>*/ {
    const recentSlot = (await provider.connection.getSlot("finalized")) - 10;

    const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;
    var [lookUpTable] = await PublicKey.findProgramAddress(
        [payerPubkey.toBuffer(), toBufferLE(BigInt(recentSlot), 8)],
        AddressLookupTableProgram.programId,
    );
    try {
      let lookUpTableRead = new PublicKey(readFileSync(path,'utf8'))
      let lookUpTableInfoInit = await provider.connection.getAccountInfo(
        lookUpTableRead
    )
      if (lookUpTableInfoInit) {
        lookUpTable = lookUpTableRead
      }
    } catch(e) {
      console.log(e)
    }

    let LOOK_UP_TABLE = await initLookUpTable(provider, lookUpTable, recentSlot);
    
    writeFile(path, LOOK_UP_TABLE.toString(),  function(err) {
      if (err) {
          return console.error(err);
      }
    });

    return LOOK_UP_TABLE; //new Promise((resolveOuter) => {LOOK_UP_TABLE});
}

export async function initLookUpTable(provider: Provider, lookupTableAddress: PublicKey, recentSlot: number): Promise<PublicKey> {

    var lookUpTableInfoInit = null;
    if (lookupTableAddress != undefined) {
        lookUpTableInfoInit = await provider.connection.getAccountInfo(
            lookupTableAddress
        )
    }
    
    if (lookUpTableInfoInit == null) {
        console.log("recentSlot: ", recentSlot);
        const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;


        const createInstruction = AddressLookupTableProgram.createLookupTable({
            authority: payerPubkey,
            payer: payerPubkey,
            recentSlot,
        })[0];
        let escrows = (await PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("escrow")],
            verifierProgramZero.programId))[0];

        let ix0 = SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_0000});

        var transaction = new Transaction().add(createInstruction);
        const addressesToAdd = [
            AUTHORITY,
            SystemProgram.programId,
            merkleTreeProgram.programId,
            DEFAULT_PROGRAMS.rent,
            PRE_INSERTED_LEAVES_INDEX,
            TOKEN_PROGRAM_ID,
            REGISTERED_POOL_PDA_SPL_TOKEN,
            MERKLE_TREE_KEY,
            escrows,
            TOKEN_AUTHORITY,
            REGISTERED_POOL_PDA_SOL
        ];
        const extendInstruction = AddressLookupTableProgram.extendLookupTable({
            lookupTable: lookupTableAddress,
            authority: payerPubkey,
            payer: payerPubkey,
            addresses: addressesToAdd,
        });

        transaction.add(extendInstruction);
        transaction.add(ix0);
        // transaction.add(ix1);
        let recentBlockhash = await provider.connection.getRecentBlockhash("confirmed");
        transaction.feePayer = payerPubkey;
        transaction.recentBlockhash = recentBlockhash;

        try {
            await sendAndConfirmTransaction(provider.connection, transaction, [ADMIN_AUTH_KEYPAIR], {commitment: "finalized", preflightCommitment: 'finalized',});
        } catch(e) {
            console.log("e : ", e);
        }

        console.log("lookupTableAddress: ", lookupTableAddress.toBase58());
        let lookupTableAccount = await provider.connection.getAccountInfo(lookupTableAddress, "confirmed");
        assert(lookupTableAccount != null);
    }
    return lookupTableAddress;

}
