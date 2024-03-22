// TODO: add
import { describe, it, expect, beforeAll } from 'vitest';
import { Connection, PublicKey, Signer, Keypair } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { bn, defaultTestStateTreeAccounts } from '@lightprotocol/stateless.js';
import { createMint, mintTo } from '../../src/actions';
import { getCompressedTokenAccountsFromMockRpc } from '../../src/get-compressed-token-accounts';

const TEST_TOKEN_DECIMALS = 2;

describe('mintTo', () => {
    // let connection: Connection;
    // let payer: Signer;
    // let bob: Signer;
    // let mint: PublicKey;
    // let mintAuthority: Keypair;
    // const { merkleTree } = defaultTestStateTreeAccounts();

    // beforeAll(async () => {
    //     connection = getConnection();
    //     payer = await newAccountWithLamports(connection);
    //     mintAuthority = Keypair.generate();
    //     const mintKeypair = Keypair.generate();

    //     mint = (
    //         await createMint(
    //             connection,
    //             payer,
    //             mintAuthority.publicKey,
    //             TEST_TOKEN_DECIMALS,
    //             mintKeypair,
    //         )
    //     ).mint;
    // });

    it('should mint to bob', async () => {});
});
