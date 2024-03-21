import {
    AccountMeta,
    PublicKey,
    TransactionInstruction,
} from '@solana/web3.js';
import { Utxo, UtxoSerde, UtxoWithMerkleContext, bn } from '../state';
import { pushUniqueItems, toArray } from '../utils/conversion';
import { LightSystemProgram } from '../programs/compressed-pda';
import { ValidityProof, checkValidityProofShape } from './validity-proof';
import { BN } from '@coral-xyz/anchor';
/// TODO: from static anchor idl
export interface InstructionDataTransfer2 {
    proofA: number[];
    proofB: number[];
    proofC: number[];
    lowElementIndices: number[];
    rootIndices: number[];
    relayFee: BN | null; // TODO: ideally bigint
    utxos: UtxoSerde;
}

/** Instruction context for state  */
export type InputState = {
    /** The utxos describing the state that is to be consumed  */
    inputUtxos: UtxoWithMerkleContext[];
    /** The indices of the state roots of the input utxos */
    inputStateRootIndices: number[];
    inputnullifierQueueAccounts: PublicKey[];
};

/** Instruction context for state' */
export type NewStateParams = {
    /** utxos describing state' */
    outputUtxos: Utxo[];
    /**
     * The pubkeys of the state trees that the utxos should be inserted into If
     * undefined, the utxos are inserted into the state tree of the 1st input utxo
     */
    outputMerkleTrees?: PublicKey[];
};

/** Format instruction data struct to align with anchor idl */
const rawInstructionData = (
    inputUtxos: UtxoWithMerkleContext[],
    recentInputStateRootIndices: number[],
    recentValidityProof: ValidityProof,
    serializedUtxos: UtxoSerde,
): InstructionDataTransfer2 => {
    return {
        proofA: Array.from(recentValidityProof.proofA),
        proofB: Array.from(recentValidityProof.proofB),
        proofC: Array.from(recentValidityProof.proofC),
        lowElementIndices: inputUtxos.map(_ => 0), // TODO: impl.!
        rootIndices: recentInputStateRootIndices,
        relayFee: bn(0),
        utxos: serializedUtxos,
    };
};

/** Pad output state trees with default tree */
export function padOutputStateTrees(
    outputStateTrees: PublicKey[] | undefined,
    defaultTree: PublicKey,
    length: number,
): PublicKey[] {
    if (!outputStateTrees || outputStateTrees.length < length) {
        const paddedTrees = new Array(length).fill(defaultTree);
        if (outputStateTrees) {
            outputStateTrees.forEach((tree, index) => {
                paddedTrees[index] = tree;
            });
        }
        return paddedTrees;
    }
    return outputStateTrees;
}

/**
 * Compresses instruction data.
 */
export interface PackInstructionParams {
    /** Utxos describing the current state to be consumed in the instruction */
    inputState: UtxoWithMerkleContext[] | UtxoWithMerkleContext;
    /** Utxos describing the new state that is to be created */
    outputState: Utxo[] | Utxo;
    /** The indices of the state roots of the input utxos. Expire with
     * validityProof */
    recentInputStateRootIndices: number[];
    /** A recent validity proof for the input state */
    recentValidityProof: ValidityProof;
    /** Optional state trees that the new state should be inserted into. Defaults
     * to 1st state tree of input state */
    outputStateTrees?: PublicKey[];
    /** TODO: account for separate signers */
    payer: PublicKey;
    /** static acccounts  */
    staticAccounts: PublicKey[];
}

/**
 * Compresses instruction data TODO: This should be usable for custom
 * instruction creation Ideally, we'd be able to pack the custom ixdata e.g.
 * create the ix but allow for more ixdata, programid etc
 *
 * TODO:
 * - refactor packInstruction -- allow separate payer/signers (all signers must
 *   be known upfront. (or at least the number of signers)) -- check if can
 *   replace coder with sync operation -- check how we can better set
 *   writable/signer for static keys -- refactor UtxoSerde to have lowlevel
 *   helper function -- we'd want a addRecentValidityProof/addRecentRootIndices
 *   helper that let's developer recompile the packed instruction without having
 *   to pass all inputs again (akin to how you can re-sign a tx with a fresh
 *   blockhash in web3js)
 */
export async function packInstruction(
    params: PackInstructionParams,
): Promise<TransactionInstruction> {
    /// validate params
    checkValidityProofShape(params.recentValidityProof);

    const inputUtxos = toArray<UtxoWithMerkleContext>(params.inputState);
    const outputUtxos = toArray<Utxo>(params.outputState);

    /// pad output state trees with 1st input state tree
    const outputStateTrees = padOutputStateTrees(
        params.outputStateTrees,
        inputUtxos[0].merkleTree,
        outputUtxos.length,
    );

    /// map unique accounts
    const remainingAccounts: PublicKey[] = [];
    const inputMerkleTrees = inputUtxos.map(utxo => utxo.merkleTree);
    const nullifierQueues = inputUtxos.map(utxo => utxo.nullifierQueue);

    pushUniqueItems<PublicKey>(inputMerkleTrees, remainingAccounts);
    pushUniqueItems<PublicKey>(nullifierQueues, remainingAccounts);
    pushUniqueItems<PublicKey>(outputStateTrees, remainingAccounts);

    const remainingAccountMetas = remainingAccounts.map(
        (account): AccountMeta => ({
            pubkey: account,
            isWritable: true, // TODO: check if inputmerkletrees should write
            isSigner: false,
        }),
    );

    /// combine static and remaining accounts
    const staticAccounts = [params.payer, ...params.staticAccounts];
    const staticAccountMetas = staticAccounts.map(
        (account): AccountMeta => ({
            pubkey: account,
            isWritable: false,
            isSigner: true, // signers
        }),
    );
    const allAccounts = [...staticAccounts, ...remainingAccounts];
    const leafIndices = inputUtxos.map(utxo => utxo.leafIndex);

    const serializedUtxos = new UtxoSerde()
        .addinputUtxos(
            inputUtxos,
            allAccounts,
            leafIndices.map(i => bn(i)),
            inputMerkleTrees,
            nullifierQueues,
        )
        .addoutputUtxos(
            outputUtxos,
            allAccounts,
            remainingAccounts,
            outputStateTrees,
        );

    /// make instruction data
    let rawInputs: InstructionDataTransfer2 = rawInstructionData(
        inputUtxos,
        params.recentInputStateRootIndices,
        params.recentValidityProof,
        serializedUtxos,
    );

    // TODO: replace native ts types bigints with BN or change to beet
    //serialization. convert to BN to support anchor encoding
    rawInputs = {
        ...rawInputs,
        //@ts-ignore
        utxos: {
            ...rawInputs.utxos,
            //@ts-ignore
            u64Array: rawInputs.utxos.u64Array.map(
                item => new BN(item.toString()),
            ),
        },
        //@ts-ignore
        relayFee: rawInputs.relayFee
            ? new BN(rawInputs.relayFee.toString())
            : new BN(0),
    };

    const data = await LightSystemProgram.program.coder.accounts.encode(
        'instructionDataTransfer2',
        rawInputs,
    );

    return new TransactionInstruction({
        keys: [...staticAccountMetas, ...remainingAccountMetas],
        data,
        programId: LightSystemProgram.programId,
    });
}
