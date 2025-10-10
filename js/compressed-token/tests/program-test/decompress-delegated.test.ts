import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Rpc,
    bn,
    TreeInfo,
    selectStateTreeInfo,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { NobleHasherFactory } from '@lightprotocol/program-test';
import {
    createMint,
    mintTo,
    approve,
    decompressDelegated,
} from '../../src/actions';
import {
    createLiteSVMRpc,
    newAccountWithLamports,
    splCreateAssociatedTokenAccount,
} from '@lightprotocol/program-test';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

interface BalanceInfo {
    delegate: ParsedTokenAccount[];
    owner: ParsedTokenAccount[];
    recipient: { value: { amount: string } };
}

async function getBalances(
    rpc: Rpc,
    delegate: PublicKey,
    owner: PublicKey,
    recipient: PublicKey,
    mint: PublicKey,
): Promise<BalanceInfo> {
    const recipientBalance = await rpc.getTokenAccountBalance(recipient);
    console.log(
        '[TEST] getBalances - recipientBalance:',
        JSON.stringify(recipientBalance),
    );
    console.log(
        '[TEST] getBalances - recipientBalance.value:',
        recipientBalance.value,
    );
    console.log(
        '[TEST] getBalances - recipientBalance.value.amount:',
        recipientBalance.value?.amount,
    );
    return {
        delegate: (
            await rpc.getCompressedTokenAccountsByDelegate(delegate, { mint })
        ).items,
        owner: (await rpc.getCompressedTokenAccountsByOwner(owner, { mint }))
            .items,
        recipient: recipientBalance,
    };
}

function calculateBalanceSum(accounts: ParsedTokenAccount[]): BN {
    return accounts.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );
}

async function assertDecompress(
    rpc: Rpc,
    initialBalances: BalanceInfo,
    recipient: PublicKey,
    mint: PublicKey,
    amount: BN,
    delegate: PublicKey,
    owner: PublicKey,
) {
    const finalBalances = await getBalances(
        rpc,
        delegate,
        owner,
        recipient,
        mint,
    );

    // Check recipient balance
    // Defensive type conversion: ensure amount is always a string before passing to bn()
    console.log(
        '[TEST] assertDecompress - initialBalances.recipient.value.amount:',
        typeof initialBalances.recipient.value.amount,
        initialBalances.recipient.value.amount,
    );
    console.log(
        '[TEST] assertDecompress - finalBalances.recipient.value.amount:',
        typeof finalBalances.recipient.value.amount,
        finalBalances.recipient.value.amount,
    );
    console.log('[TEST] assertDecompress - amount:', amount.toString());

    const expectedRecipientBalance = bn(
        String(initialBalances.recipient.value.amount),
    ).add(amount);
    const actualRecipientBalance = bn(
        String(finalBalances.recipient.value.amount),
    );

    console.log(
        '[TEST] assertDecompress - expectedRecipientBalance:',
        expectedRecipientBalance.toString(),
    );
    console.log(
        '[TEST] assertDecompress - actualRecipientBalance:',
        actualRecipientBalance.toString(),
    );

    expect(actualRecipientBalance.toString()).toBe(
        expectedRecipientBalance.toString(),
    );

    // Check delegate and owner balances
    const initialDelegateSum = calculateBalanceSum(initialBalances.delegate);
    const finalDelegateSum = calculateBalanceSum(finalBalances.delegate);
    const finalOwnerSum = calculateBalanceSum(finalBalances.owner);

    expect(finalDelegateSum.add(finalOwnerSum).toString()).toBe(
        initialDelegateSum.sub(amount).toString(),
    );
}

const TEST_TOKEN_DECIMALS = 2;
const TEST_AMOUNT = bn(5);
const INITIAL_MINT_AMOUNT = bn(1000);

describe('decompressDelegated', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let charlieAta: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        const lightWasm = await NobleHasherFactory.getInstance();
        rpc = await createLiteSVMRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
        bob = await newAccountWithLamports(rpc, 1e9);
        charlie = await newAccountWithLamports(rpc, 1e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);

        charlieAta = await splCreateAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            charlie.publicKey,
        );

        const randInfo = selectTokenPoolInfo(tokenPoolInfos);
        await mintTo(
            rpc,
            payer,
            mint,
            payer.publicKey,
            mintAuthority,
            INITIAL_MINT_AMOUNT,
            stateTreeInfo,
            randInfo,
        );

        await approve(
            rpc,
            payer,
            mint,
            INITIAL_MINT_AMOUNT.toNumber(),
            payer,
            bob.publicKey,
        );
    });

    it('should decompress from bob -> charlieAta and leave no delegated remainder', async () => {
        const initialBalances = await getBalances(
            rpc,
            bob.publicKey,
            payer.publicKey,
            charlieAta,
            mint,
        );
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);

        await decompressDelegated(
            rpc,
            bob,
            mint,
            TEST_AMOUNT,
            bob,
            charlieAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, TEST_AMOUNT),
        );

        await assertDecompress(
            rpc,
            initialBalances,
            charlieAta,
            mint,
            TEST_AMOUNT,
            bob.publicKey,
            payer.publicKey,
        );
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);

        await expect(
            decompressDelegated(
                rpc,
                bob,
                mint,
                TEST_AMOUNT,
                bob,
                charlieAta,
                selectTokenPoolInfosForDecompression(
                    tokenPoolInfos,
                    TEST_AMOUNT,
                ),
            ),
        ).rejects.toThrowError(
            'Could not find accounts to select for transfer.',
        );
    });
});
