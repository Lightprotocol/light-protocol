import { describe, it, expect } from 'vitest';
import {
    createCompressedAccountLegacy,
    createCompressedAccountWithMerkleContextLegacy,
    createMerkleContextLegacy,
} from '../../../src/state/compressed-account';
import { PublicKey } from '@solana/web3.js';
import { bn } from '../../../src/state';
import { TreeType } from '../../../src/state';

describe('createCompressedAccount function', () => {
    it('should create a compressed account with default values', () => {
        const owner = PublicKey.unique();
        const account = createCompressedAccountLegacy(owner);
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
        const account = createCompressedAccountLegacy(
            owner,
            lamports,
            data,
            address,
        );
        expect(account).toEqual({
            owner,
            lamports,
            address,
            data,
        });
    });
});

describe('createCompressedAccountWithMerkleContextLegacy function', () => {
    it('should create a compressed account with merkle context', () => {
        const owner = PublicKey.unique();
        const merkleTree = PublicKey.unique();
        const nullifierQueue = PublicKey.unique();
        const hash = new Array(32).fill(1);
        const leafIndex = 0;
        const merkleContext = createMerkleContextLegacy(
            {
                tree: merkleTree,
                queue: nullifierQueue,
                treeType: TreeType.AddressV1,
                nextTreeInfo: null,
            },
            bn(hash),
            leafIndex,
        );
        const accountWithMerkleContext =
            createCompressedAccountWithMerkleContextLegacy(
                merkleContext,
                owner,
            );
        expect(accountWithMerkleContext).toEqual({
            owner,
            lamports: bn(0),
            address: null,
            data: null,
            treeInfo: {
                tree: merkleTree,
                queue: nullifierQueue,
                treeType: TreeType.AddressV1,
                nextTreeInfo: null,
            },
            hash: bn(hash),
            leafIndex,
            readOnly: false,
            proveByIndex: false,
        });
    });
});

describe('createMerkleContext function', () => {
    it('should create a merkle context', () => {
        const merkleTree = PublicKey.unique();
        const nullifierQueue = PublicKey.unique();
        const hash = new Array(32).fill(1);

        const leafIndex = 0;
        const merkleContext = createMerkleContextLegacy(
            {
                tree: merkleTree,
                queue: nullifierQueue,
                treeType: TreeType.AddressV1,
                nextTreeInfo: null,
            },
            bn(hash),
            leafIndex,
            false,
        );
        expect(merkleContext).toEqual({
            treeInfo: {
                tree: merkleTree,
                queue: nullifierQueue,
                treeType: TreeType.AddressV1,
                nextTreeInfo: null,
            },
            hash: bn(hash),
            leafIndex,
            proveByIndex: false,
        });
    });
});
