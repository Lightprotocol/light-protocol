import * as anchor from "@coral-xyz/anchor";
import { IDL_VERIFIER_PROGRAM_ONE, IDL_VERIFIER_PROGRAM_TWO, IDL_VERIFIER_PROGRAM_ZERO, IDL_VERIFIER_PROGRAM_STORAGE, } from "../idls/index";
import { ADMIN_AUTH_KEYPAIR, POOL_TYPE, MINT, verifierProgramZeroProgramId, verifierProgramOneProgramId, verifierProgramTwoProgramId, verifierProgramStorageProgramId, Transaction, merkleTreeProgramId, airdropSol, } from "../index";
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig";
export async function setUpMerkleTree(provider, merkleTreeAuthority) {
    let merkleTreeConfig = new MerkleTreeConfig({
        payer: ADMIN_AUTH_KEYPAIR,
        connection: provider.connection,
    });
    console.log(await merkleTreeConfig.getMerkleTreeAuthorityPda());
    console.log(await provider.connection.getAccountInfo(await merkleTreeConfig.getMerkleTreeAuthorityPda()));
    if ((await provider.connection.getAccountInfo(await merkleTreeConfig.getMerkleTreeAuthorityPda())) == null) {
        await merkleTreeConfig.initMerkleTreeAuthority();
    }
    else {
        console.log("was already executed: initMerkleTreeAuthority");
    }
    if ((await provider.connection.getAccountInfo(MerkleTreeConfig.getEventMerkleTreePda())) == null) {
        await merkleTreeConfig.initializeNewEventMerkleTree();
    }
    else {
        console.log("was already executed: initializeNewEventMerkleTree");
    }
    if ((await provider.connection.getAccountInfo((await merkleTreeConfig.getPoolTypePda(POOL_TYPE)).poolPda)) == null) {
        await merkleTreeConfig.registerPoolType(POOL_TYPE);
    }
    else {
        console.log("was already executed: registerPoolType");
    }
    if ((await provider.connection.getAccountInfo((await merkleTreeConfig.getSplPoolPda(MINT, POOL_TYPE)).pda)) == null) {
        await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT);
    }
    else {
        console.log("was already executed: registerSplPool");
    }
    if ((await provider.connection.getAccountInfo(MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId, POOL_TYPE).pda)) == null) {
        await merkleTreeConfig.registerSolPool(POOL_TYPE);
    }
    else {
        console.log("was already executed: registerSolPool");
    }
    // TODO: do verifier registry in constants
    const verifierArray = [];
    verifierArray.push(new anchor.Program(IDL_VERIFIER_PROGRAM_ZERO, verifierProgramZeroProgramId));
    verifierArray.push(new anchor.Program(IDL_VERIFIER_PROGRAM_ONE, verifierProgramOneProgramId));
    verifierArray.push(new anchor.Program(IDL_VERIFIER_PROGRAM_TWO, verifierProgramTwoProgramId));
    verifierArray.push(new anchor.Program(IDL_VERIFIER_PROGRAM_STORAGE, verifierProgramStorageProgramId));
    // registering verifiers and airdrop sol to authority pdas
    for (var verifier of verifierArray) {
        const pda = (await merkleTreeConfig.getRegisteredVerifierPda(verifier.programId)).registeredVerifierPda;
        if ((await provider.connection.getAccountInfo(pda)) == null) {
            await merkleTreeConfig.registerVerifier(verifier.programId);
        }
        else {
            console.log(`verifier ${verifier.programId.toBase58()} is already initialized`);
        }
        const authorityPda = Transaction.getSignerAuthorityPda(merkleTreeProgramId, verifier.programId);
        await airdropSol({
            connection: provider.connection,
            lamports: 1000000000,
            recipientPublicKey: authorityPda,
        });
        console.log(`Registering Verifier ${verifier.programId.toBase58()}, pda ${pda.toBase58()} and funded authority pda success ${authorityPda.toBase58()}`);
    }
    if (merkleTreeAuthority) {
        await merkleTreeConfig.updateMerkleTreeAuthority(merkleTreeAuthority, true);
    }
}
//# sourceMappingURL=setUpMerkleTree.js.map