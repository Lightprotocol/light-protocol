import { IndexedArray } from '../../../src/test-helpers/merkle-tree/indexed-array';
import { beforeAll, describe, expect, it } from 'vitest';
import { IndexedElement } from '../../../src/test-helpers/merkle-tree/indexed-array';
import { HIGHEST_ADDRESS_PLUS_ONE } from '../../../src/constants';
import { bn } from '../../../src/state';
import { MerkleTree } from '../../../src/test-helpers/merkle-tree';

describe('MerkleTree', () => {
    let WasmFactory: any;
    const refIndexedMerkleTreeInitedRoot = [
        33, 133, 56, 184, 142, 166, 110, 161, 4, 140, 169, 247, 115, 33, 15,
        181, 76, 89, 48, 126, 58, 86, 204, 81, 16, 121, 185, 77, 75, 152, 43,
        15,
    ];

    const refIndexedMerkleTreeRootWithOneAppend = [
        31, 159, 196, 171, 68, 16, 213, 28, 158, 200, 223, 91, 244, 193, 188,
        162, 50, 68, 54, 244, 116, 44, 153, 65, 209, 9, 47, 98, 126, 89, 131,
        158,
    ];

    const refIndexedMerkleTreeRootWithTwoAppends = [
        1, 185, 99, 233, 59, 202, 51, 222, 224, 31, 119, 180, 76, 104, 72, 27,
        152, 12, 236, 78, 81, 60, 87, 158, 237, 1, 176, 9, 155, 166, 108, 89,
    ];
    const refIndexedMerkleTreeRootWithThreeAppends = [
        41, 143, 181, 2, 66, 117, 37, 226, 134, 212, 45, 95, 114, 60, 189, 18,
        44, 155, 132, 148, 41, 54, 131, 106, 61, 120, 237, 168, 118, 198, 63,
        116,
    ];

    const refIndexedArrayElem0 = new IndexedElement(0, bn(0), 2);
    const refIndexedArrayElem1 = new IndexedElement(
        1,
        HIGHEST_ADDRESS_PLUS_ONE,
        0,
    );
    const refIndexedArrayElem2 = new IndexedElement(2, bn(30), 1);

    describe('IndexedArray', () => {
        beforeAll(async () => {
            WasmFactory = (await import('@lightprotocol/hasher.rs'))
                .WasmFactory;
        });

        it('should findLowElementIndex', () => {
            const indexedArray = new IndexedArray(
                [
                    refIndexedArrayElem0,
                    refIndexedArrayElem1,
                    refIndexedArrayElem2,
                ],
                2,
                1,
            );
            expect(indexedArray.findLowElementIndex(bn(29))).toEqual(0);
            expect(() => indexedArray.findLowElementIndex(bn(30))).toThrow();
            expect(indexedArray.findLowElementIndex(bn(31))).toEqual(2);
        });

        it('should findLowElement', () => {
            const indexedArray = new IndexedArray(
                [
                    refIndexedArrayElem0,
                    refIndexedArrayElem1,
                    refIndexedArrayElem2,
                ],
                2,
                1,
            );
            const [lowElement, nextValue] = indexedArray.findLowElement(bn(29));
            expect(lowElement).toEqual(refIndexedArrayElem0);
            expect(nextValue).toEqual(bn(30));

            expect(() => indexedArray.findLowElement(bn(30))).toThrow();

            const [lowElement2, nextValue2] = indexedArray.findLowElement(
                bn(31),
            );
            expect(lowElement2).toEqual(refIndexedArrayElem2);
            expect(nextValue2).toEqual(HIGHEST_ADDRESS_PLUS_ONE);
        });

        it('should appendWithLowElementIndex', () => {
            const indexedArray = new IndexedArray(
                [
                    new IndexedElement(0, bn(0), 1),
                    new IndexedElement(1, HIGHEST_ADDRESS_PLUS_ONE, 0),
                ],
                1,
                1,
            );
            const newElement = indexedArray.appendWithLowElementIndex(
                0,
                bn(30),
            );
            expect(newElement.newElement).toEqual(refIndexedArrayElem2);
            expect(newElement.newLowElement).toEqual(refIndexedArrayElem0);
            expect(newElement.newElementNextValue).toEqual(
                HIGHEST_ADDRESS_PLUS_ONE,
            );
        });

        it('should append', () => {
            const indexedArray = new IndexedArray(
                [
                    new IndexedElement(0, bn(0), 1),
                    new IndexedElement(1, HIGHEST_ADDRESS_PLUS_ONE, 0),
                ],
                1,
                1,
            );
            const newElement = indexedArray.append(bn(30));
            expect(newElement.newElement).toEqual(refIndexedArrayElem2);
            expect(newElement.newLowElement).toEqual(refIndexedArrayElem0);
            expect(newElement.newElementNextValue).toEqual(
                HIGHEST_ADDRESS_PLUS_ONE,
            );
        });

        it('should append 3 times and match merkle trees', async () => {
            const lightWasm = await WasmFactory.getInstance();

            const indexedArray = IndexedArray.default();
            indexedArray.init();

            let hash0 = indexedArray.hashElement(lightWasm, 0);
            let hash1 = indexedArray.hashElement(lightWasm, 1);
            let leaves = [hash0, hash1].map(leaf => bn(leaf!).toString());
            let tree = new MerkleTree(26, lightWasm, leaves);
            expect(tree.root()).toEqual(
                bn(refIndexedMerkleTreeInitedRoot).toString(),
            );

            // 1st
            const newElement = indexedArray.append(bn(30));
            expect(newElement.newElement).toEqual(refIndexedArrayElem2);
            expect(newElement.newLowElement).toEqual(refIndexedArrayElem0);
            expect(newElement.newElementNextValue).toEqual(
                HIGHEST_ADDRESS_PLUS_ONE,
            );
            hash0 = indexedArray.hashElement(lightWasm, 0);
            hash1 = indexedArray.hashElement(lightWasm, 1);
            let hash2 = indexedArray.hashElement(lightWasm, 2);
            leaves = [hash0, hash1, hash2].map(leaf => bn(leaf!).toString());
            tree = new MerkleTree(26, lightWasm, leaves);
            expect(tree.root()).toEqual(
                bn(refIndexedMerkleTreeRootWithOneAppend).toString(),
            );

            // 2nd
            let refItems0 = new IndexedElement(0, bn(0), 2);
            let refItems1 = new IndexedElement(1, HIGHEST_ADDRESS_PLUS_ONE, 0);
            let refItems2 = new IndexedElement(2, bn(30), 3);
            let refItems3 = new IndexedElement(3, bn(42), 1);

            const newElement2 = indexedArray.append(bn(42));

            expect(newElement2.newElement).toEqual(refItems3);
            expect(newElement2.newLowElement).toEqual(refItems2);
            expect(newElement2.newElementNextValue).toEqual(
                HIGHEST_ADDRESS_PLUS_ONE,
            );
            expect(indexedArray.elements[0].equals(refItems0)).toBeTruthy();
            expect(indexedArray.elements[1].equals(refItems1)).toBeTruthy();
            expect(indexedArray.elements[2].equals(refItems2)).toBeTruthy();
            expect(indexedArray.elements[3].equals(refItems3)).toBeTruthy();

            hash0 = indexedArray.hashElement(lightWasm, 0);
            hash1 = indexedArray.hashElement(lightWasm, 1);
            hash2 = indexedArray.hashElement(lightWasm, 2);
            let hash3 = indexedArray.hashElement(lightWasm, 3);
            leaves = [hash0, hash1, hash2, hash3].map(leaf =>
                bn(leaf!).toString(),
            );
            tree = new MerkleTree(26, lightWasm, leaves);

            expect(tree.root()).toEqual(
                bn(refIndexedMerkleTreeRootWithTwoAppends).toString(),
            );

            // 3rd
            refItems0 = new IndexedElement(0, bn(0), 4);
            refItems1 = new IndexedElement(1, HIGHEST_ADDRESS_PLUS_ONE, 0);
            refItems2 = new IndexedElement(2, bn(30), 3);
            refItems3 = new IndexedElement(3, bn(42), 1);
            const refItems4 = new IndexedElement(4, bn(12), 2);

            const newElement3 = indexedArray.append(bn(12));
            expect(newElement3.newElement).toEqual(refItems4);
            expect(newElement3.newLowElement).toEqual(refItems0);
            expect(newElement3.newElementNextValue).toEqual(bn(30));
            expect(indexedArray.elements[0].equals(refItems0)).toBeTruthy();
            expect(indexedArray.elements[1].equals(refItems1)).toBeTruthy();
            expect(indexedArray.elements[2].equals(refItems2)).toBeTruthy();
            expect(indexedArray.elements[3].equals(refItems3)).toBeTruthy();
            expect(indexedArray.elements[4].equals(refItems4)).toBeTruthy();

            hash0 = indexedArray.hashElement(lightWasm, 0);
            hash1 = indexedArray.hashElement(lightWasm, 1);
            hash2 = indexedArray.hashElement(lightWasm, 2);
            hash3 = indexedArray.hashElement(lightWasm, 3);
            const hash4 = indexedArray.hashElement(lightWasm, 4);
            leaves = [hash0, hash1, hash2, hash3, hash4].map(leaf =>
                bn(leaf!).toString(),
            );
            tree = new MerkleTree(26, lightWasm, leaves);

            expect(tree.root()).toEqual(
                bn(refIndexedMerkleTreeRootWithThreeAppends).toString(),
            );
        });
    });
});
