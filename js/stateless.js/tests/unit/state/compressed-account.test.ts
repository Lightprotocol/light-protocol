import { describe, it, expect } from 'vitest';
import {
    createCompressedAccount,
    createCompressedAccountWithMerkleContext,
    createMerkleContext,
} from '../../../src/state/compressed-account';
import { PublicKey } from '@solana/web3.js';
import { bn } from '../../../src/state/BN254';
import { TreeType } from '../../../src/state';

describe('createCompressedAccount function', () => {
    it('should create a compressed account with default values', () => {
        const owner = PublicKey.unique();
        const account = createCompressedAccount(owner);
        expect(account).toEqual({
            owner,
            lamports: bn(0),
            address: null,
            data: null,
        });
    });

    it('should create a compressed account with provided values', () => {
        const owner = PublicKey.unique();
        const lamports = bn(100);
        const data = {
            discriminator: [0],
            data: Buffer.from(new Uint8Array([1, 2, 3])),
            dataHash: [0],
        };
        const address = Array.from(PublicKey.unique().toBytes());
        const account = createCompressedAccount(owner, lamports, data, address);
        expect(account).toEqual({
            owner,
            lamports,
            address,
            data,
        });
    });
});

describe('createCompressedAccountWithMerkleContext function', () => {
    it('should create a compressed account with merkle context', () => {
        const owner = PublicKey.unique();
        const merkleTree = PublicKey.unique();
        const queue = PublicKey.unique();
        const hash = new Array(32).fill(1);
        const leafIndex = 0;
        const merkleContext = createMerkleContext(
            merkleTree,
            queue,
            hash,
            leafIndex,
            TreeType.StateV1,
            false,
        );
        const accountWithMerkleContext =
            createCompressedAccountWithMerkleContext(merkleContext, owner);
        expect(accountWithMerkleContext).toEqual({
            owner,
            lamports: bn(0),
            address: null,
            data: null,
            merkleTree,
            queue,
            hash,
            leafIndex,
            treeType: TreeType.StateV1,
            proveByIndex: false,
            readOnly: false,
        });
    });
});

describe('createMerkleContext function', () => {
    it('should create a merkle context', () => {
        const merkleTree = PublicKey.unique();
        const queue = PublicKey.unique();
        const hash = new Array(32).fill(1);

        const leafIndex = 0;
        const merkleContext = createMerkleContext(
            merkleTree,
            queue,
            hash,
            leafIndex,
            TreeType.StateV1,
            false,
        );
        expect(merkleContext).toEqual({
            merkleTree,
            queue,
            hash,
            leafIndex,
            treeType: TreeType.StateV1,
            proveByIndex: false,
        });
    });
});
