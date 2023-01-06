import * as anchor from "@project-serum/anchor";
import { assert } from "chai";

import {
    MERKLE_TREE_KEY,
    ADMIN_AUTH_KEYPAIR,
    AUTHORITY,
    merkleTreeProgram,
    verifierProgramZero,
    verifierProgramOne,
    verifierProgramTwo,
    MINT_PRIVATE_KEY,
    POOL_TYPE,
    MINT,
    REGISTERED_POOL_PDA_SPL,
    REGISTERED_POOL_PDA_SOL,
    MERKLE_TREE_AUTHORITY_PDA,
    confirmConfig,
} from "../constants"
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig"

import {createMint } from "./createAccounts"

export async function setUpMerkleTree (provider: anchor.Provider) {
    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
        MERKLE_TREE_KEY
    )
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    console.log("MERKLE_TREE_KEY ", MERKLE_TREE_KEY);
    console.log("ADMIN_AUTH_KEYPAIR ", ADMIN_AUTH_KEYPAIR);
    
    if (merkleTreeAccountInfoInit == null) {
    
    let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })
    
    console.log("Initing MERKLE_TREE_AUTHORITY_PDA");
    
    try {
        const ix = await merkleTreeConfig.initMerkleTreeAuthority();
        console.log("initMerkleTreeAuthority success, ", ix);
        // assert(await provider.connection.getTransaction(ix, {commitment:"confirmed"}) != null, "init failed");
    } catch(e) {
        console.log(e);
    }
    
    console.log("AUTHORITY: ", AUTHORITY);
    
    console.log("AUTHORITY: ", Array.from(AUTHORITY.toBytes()));
    
    console.log("verifierProgramZero.programId: ", Array.from(verifierProgramZero.programId.toBytes()));
    console.log("MERKLE_TREE_KEY: ", MERKLE_TREE_KEY.toBase58())
    console.log("MERKLE_TREE_KEY: ", Array.from(MERKLE_TREE_KEY.toBytes()))
    // console.log("MERKLE_TREE_PDA_TOKEN: ", MERKLE_TREE_PDA_TOKEN.toBase58())
    // console.log("MERKLE_TREE_PDA_TOKEN: ", Array.from(MERKLE_TREE_PDA_TOKEN.toBytes()))
    
    try {
        const ix = await merkleTreeConfig.initializeNewMerkleTree()
        assert(await provider.connection.getTransaction(ix, {commitment:"confirmed"}) != null, "init failed");

    } catch(e) {
        console.log(e);
    }
    
    console.log("Registering Verifier");
    try {
        await merkleTreeConfig.registerVerifier(verifierProgramZero.programId)
        console.log("Registering Verifier Zero success");
    } catch(e) {
        console.log(e);
    }
    
    try {
        await merkleTreeConfig.registerVerifier(verifierProgramOne.programId)
        console.log("Registering Verifier One success");
    } catch(e) {
        console.log(e);
    
    }
    
    try {
        await merkleTreeConfig.registerVerifier(verifierProgramTwo.programId)
        console.log("Registering Verifier One success");
    } catch(e) {
        console.log(e);
    }    
    
    try {
        await merkleTreeConfig.registerPoolType(POOL_TYPE)
        console.log("Registering pool_type success");
    } catch(e) {
        console.log(e);
    }
    
    console.log("MINT: ", MINT.toBase58());
    console.log("POOL_TYPE_PDA: ", REGISTERED_POOL_PDA_SPL.toBase58());
    try {
        await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT)
        console.log("Registering spl pool success");
    } catch(e) {
        console.log(e);
    }
    
    console.log("REGISTERED_POOL_PDA_SOL: ", REGISTERED_POOL_PDA_SOL);
    try {
        await merkleTreeConfig.registerSolPool(POOL_TYPE)
        console.log("Registering sol pool success");
    } catch(e) {
        console.log(e);
    }
    }
}
