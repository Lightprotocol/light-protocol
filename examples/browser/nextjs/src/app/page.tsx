'use client';
import {
    LightSystemProgram,
    bn,
    buildTx,
    confirmTx,
    createRpc,
    defaultTestStateTreeAccounts,
    selectMinCompressedSolAccountsForTransfer,
} from '@lightprotocol/stateless.js';
import {
    ComputeBudgetProgram,
    Keypair,
    TransactionMessage,
    VersionedTransaction,
} from '@solana/web3.js';
import React, { FC, useCallback, useMemo } from 'react';
import {
    ConnectionProvider,
    WalletProvider,
    useWallet,
} from '@solana/wallet-adapter-react';
import { WalletNotConnectedError } from '@solana/wallet-adapter-base';
import { UnsafeBurnerWalletAdapter } from '@solana/wallet-adapter-unsafe-burner';
import {
    WalletModalProvider,
    WalletDisconnectButton,
    WalletMultiButton,
} from '@solana/wallet-adapter-react-ui';

// Default styles that can be overridden by your app
require('@solana/wallet-adapter-react-ui/styles.css');

export default function Home() {
    const endpoint = useMemo(() => 'http://127.0.0.1:8899', []);
    const wallets = useMemo(() => [new UnsafeBurnerWalletAdapter()], []);

    return (
        <ConnectionProvider endpoint={endpoint}>
            <WalletProvider wallets={wallets} autoConnect>
                <WalletModalProvider>
                    <WalletMultiButton />
                    <WalletDisconnectButton />
                    <div>
                        <label>Check the terminal for tx signatures!</label>
                        <SendButton />
                    </div>
                </WalletModalProvider>
            </WalletProvider>
        </ConnectionProvider>
    );
}

const SendButton: FC = () => {
    const { publicKey, sendTransaction } = useWallet();

    /// Get Connection with compatibility to Compression API
    const connection = createRpc();

    const onClick = useCallback(async () => {
        if (!publicKey) throw new WalletNotConnectedError();

        /// airdrop
        await confirmTx(
            connection,
            await connection.requestAirdrop(publicKey, 1e9),
        );

        /// compress to self
        const compressInstruction = await LightSystemProgram.compress({
            payer: publicKey,
            toAddress: publicKey,
            lamports: 1e8,
            outputStateTree: defaultTestStateTreeAccounts().merkleTree,
        });

        const compressInstructions = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            compressInstruction,
        ];

        const {
            context: { slot: minContextSlot },
            value: blockhashCtx,
        } = await connection.getLatestBlockhashAndContext();

       const tx = buildTx(publicKey, compressInstructions, blockhashCtx)

        const signature = await sendTransaction(transaction, connection, {
            minContextSlot,
        });

        await connection.confirmTransaction({
            blockhash,
            lastValidBlockHeight,
            signature,
        });

        console.log(
            `Compressed ${1e8} lamports! txId: https://explorer.solana.com/tx/${signature}?cluster=custom`,
        );
        /// Send compressed SOL to a random address
        const recipient = Keypair.generate().publicKey;

        /// 1. We need to fetch our sol balance
        const accounts =
            await connection.getCompressedAccountsByOwner(publicKey);

        console.log('accounts', accounts);
        const [selectedAccounts, _] = selectMinCompressedSolAccountsForTransfer(
            accounts,
            1e7,
        );

        console.log('selectedAccounts', selectedAccounts);
        /// 2. Retrieve validity proof for our selected balance
        const { compressedProof, rootIndices } =
            await connection.getValidityProof(
                selectedAccounts.map(account => bn(account.hash)),
            );

        /// 3. Create and send compressed transfer
        const sendInstruction = await LightSystemProgram.transfer({
            payer: publicKey,
            toAddress: recipient,
            lamports: 1e7,
            inputCompressedAccounts: selectedAccounts,
            outputStateTrees: [defaultTestStateTreeAccounts().merkleTree],
            recentValidityProof: compressedProof,
            recentInputStateRootIndices: rootIndices,
        });
        const sendInstructions = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            sendInstruction,
        ];

        const {
            context: { slot: minContextSlotSend },
            value: {
                blockhash: blockhashSend,
                lastValidBlockHeight: lastValidBlockHeightSend,
            },
        } = await connection.getLatestBlockhashAndContext();

        const messageV0Send = new TransactionMessage({
            payerKey: publicKey,
            recentBlockhash: blockhash,
            instructions: sendInstructions,
        }).compileToV0Message();

        const transactionSend = new VersionedTransaction(messageV0Send);

        const signatureSend = await sendTransaction(
            transactionSend,
            connection,
            {
                minContextSlot: minContextSlotSend,
            },
        );

        await connection.confirmTransaction({
            blockhash: blockhashSend,
            lastValidBlockHeight: lastValidBlockHeightSend,
            signature: signatureSend,
        });

        console.log(
            `Sent ${1e7} lamports to ${recipient.toBase58()} ! txId: https://explorer.solana.com/tx/${signatureSend}?cluster=custom`,
        );
    }, [publicKey, sendTransaction, connection]);

    return (
        <button onClick={onClick} disabled={!publicKey}>
            Get airdrop, compress and send SOL to a random address!
        </button>
    );
};
