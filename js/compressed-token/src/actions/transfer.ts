import {
    ConfirmOptions,
    Connection,
    PublicKey,
    Signer,
    TransactionSignature,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    bn,
    defaultTestStateTreeAccounts,
    sendAndConfirmTx,
    getMockRpc,
} from '@lightprotocol/stateless.js';
import { buildAndSignTx } from '@lightprotocol/stateless.js';
import { BN } from '@coral-xyz/anchor';
import { createTransferInstruction } from '../instructions';
import { TokenTransferOutUtxo_IdlType } from '../types';
import {
    UtxoWithParsedTokenTlvData,
    getCompressedTokenAccountsFromMockRpc,
} from '../get-compressed-token-accounts';
import { dedupeSigner, getSigners } from './common';

/**
 * @internal
 *
 * Selects the minimal number of compressed token accounts for a transfer
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
function selectMinCompressedTokenAccountsForTransfer(
    accounts: UtxoWithParsedTokenTlvData[],
    transferAmount: BN,
): [
    selectedAccounts: UtxoWithParsedTokenTlvData[],
    total: BN,
    totalLamports: BN | null,
] {
    let accumulatedAmount = bn(0);
    let accumulatedLamports = bn(0);

    const selectedAccounts: UtxoWithParsedTokenTlvData[] = [];

    accounts.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));

    for (const account of accounts) {
        if (accumulatedAmount.gte(bn(transferAmount))) break;
        accumulatedAmount = accumulatedAmount.add(account.parsed.amount);
        accumulatedLamports = accumulatedLamports.add(account.utxo.lamports);
        selectedAccounts.push(account);
    }

    if (accumulatedAmount.lt(bn(transferAmount))) {
        throw new Error('Not enough balance for transfer');
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports.lt(bn(0)) ? accumulatedLamports : null,
    ];
}

/**
 * Transfer compressed tokens from one owner to another
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint of the compressed token
 * @param amount         Number of tokens to transfer
 * @param owner          Owner of the compressed tokens
 * @param toAddress      Destination address of the recipient
 * @param merkleTree     State tree account that the compressed tokens should be
 *                       inserted into. Defaults to the default state tree account.
 * @param multiSigners   Signing accounts if `currentOwner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer | PublicKey,
    toAddress: PublicKey,
    /// TODO: pass output state trees.
    merkleTree: PublicKey = defaultTestStateTreeAccounts().merkleTree,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const [currentOwnerPublicKey, signers] = getSigners(owner, multiSigners);

    amount = bn(amount);

    const compressedTokenAccounts = await getCompressedTokenAccountsFromMockRpc(
        connection,
        currentOwnerPublicKey,
        mint,
    );

    const [inputAccounts, inputAmount, inputLamports] =
        selectMinCompressedTokenAccountsForTransfer(
            compressedTokenAccounts,
            amount,
        );

    /// Create output utxos
    const changeAmount = inputAmount.sub(amount);
    /// We don't send lamports and don't have rent
    const changeLamportsAmount = inputLamports;

    const changeUtxo: TokenTransferOutUtxo_IdlType = {
        amount: changeAmount,
        owner: currentOwnerPublicKey,
        lamports: changeLamportsAmount,
        index_mt_account: 0, // FIXME: dynamic!
    };

    const recipientOutUtxo: TokenTransferOutUtxo_IdlType = {
        amount,
        owner: toAddress,
        lamports: null,
        index_mt_account: 0, // FIXME: dynamic!
    };

    const rpc = await getMockRpc(connection);

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => account.merkleContext!.hash as BN),
    );

    const ixs = await createTransferInstruction(
        payer.publicKey,
        currentOwnerPublicKey,
        inputAccounts.map(account => account.merkleContext!.merkleTree),
        inputAccounts.map(account => account.merkleContext!.nullifierQueue),
        [merkleTree, merkleTree],
        inputAccounts.map(utxo => utxo.utxo),
        [recipientOutUtxo, changeUtxo],
        // TODO: replace with actual recent state root index!
        // This will only work with sequential state updates and no cranking!
        proof.rootIndices, // input state root indices
        proof.compressedProof,
    );

    const { blockhash } = await connection.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, signers);
    const signedTx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
    const txId = await sendAndConfirmTx(connection, signedTx, confirmOptions);

    return txId;
}
