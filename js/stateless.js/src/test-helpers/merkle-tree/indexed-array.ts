import { LightWasm } from '../test-rpc/test-rpc';
// TODO: remove import
// import { WasmFactory } from '@lightprotocol/hasher.rs';

import { BN } from '@coral-xyz/anchor';
import { bn } from '../../state';
import { MerkleTree } from './merkle-tree';
import { beforeAll } from 'vitest';

const FIELD_SIZE_SUB_ONE: string =
    '21888242871839275222246405745257275088548364400416034343698204186575808495616';

export class IndexedElement {
    public index: number;
    public value: BN;
    public nextIndex: number;

    constructor(index: number, value: BN, nextIndex: number) {
        this.index = index;
        this.value = value;
        this.nextIndex = nextIndex;
    }

    public equals(other: IndexedElement): boolean {
        return this.value.eq(other.value);
    }

    public compareTo(other: IndexedElement): number {
        return this.value.cmp(other.value);
    }

    public hash(lightWasm: LightWasm, nextValue: BN): Uint8Array {
        try {
            const hash = lightWasm.poseidonHash([
                bn(this.value.toArray('be', 32)).toString(),
                bn(this.nextIndex).toString(),
                bn(nextValue.toArray('be', 32)).toString(),
            ]);
            return hash;
            // const hash = H.hashv([
            //     bigintToBeBytesArray<32>(this.value),
            //     this.nextIndex.toBytes(),
            //     bigintToBeBytesArray<32>(nextValue),
            // ]);
            // return [hash, undefined];
        } catch (error) {
            throw new Error('Hashing failed');
            // return [0, new IndexedMerkleTreeError('Hashing failed')];
        }
    }
}

export class IndexedElementBundle {
    public newLowElement: IndexedElement;
    public newElement: IndexedElement;
    public newElementNextValue: BN;

    constructor(
        newLowElement: IndexedElement,
        newElement: IndexedElement,
        newElementNextValue: BN,
    ) {
        this.newLowElement = newLowElement;
        this.newElement = newElement;
        this.newElementNextValue = newElementNextValue;
    }
}

/**
 * This indexed array implementation mirrors the rust implementation of the
 * indexed merkle tree. It stores the elements of the indexed merkle tree.
 */
export class IndexedArray {
    public elements: Array<IndexedElement>;
    public currentNodeIndex: number;
    public highestElementIndex: number;

    constructor(
        elements: Array<IndexedElement>,
        currentNodeIndex: number,
        highestElementIndex: number,
    ) {
        this.elements = elements;
        this.currentNodeIndex = currentNodeIndex;
        this.highestElementIndex = highestElementIndex;
    }

    public static default(): IndexedArray {
        return new IndexedArray([new IndexedElement(0, bn(0), 0)], 0, 0);
    }

    public get(index: number): IndexedElement | undefined {
        return this.elements[index];
    }

    public length(): number {
        return Number(this.currentNodeIndex);
    }

    public isEmpty(): boolean {
        return this.currentNodeIndex === 0;
    }

    public findElement(value: BN): IndexedElement | undefined {
        return this.elements
            .slice(0, this.length() + 1)
            .find(node => node.value === value);
    }

    public init(): IndexedElementBundle {
        try {
            const init_value = new BN(FIELD_SIZE_SUB_ONE);
            return this.append(init_value);
        } catch (error) {
            throw new Error(`Failed to initialize IndexedArray: ${error}`);
        }
    }

    /**
     * Finds the index of the low element for the given `value` which should not be part of the array.
     * Low element is the greatest element which still has a lower value than the provided one.
     * Low elements are used in non-membership proofs.
     */
    public findLowElementIndex(value: BN): number | undefined {
        // Try to find element whose next element is higher than the provided value.
        for (let i = 0; i <= this.length(); i++) {
            const node = this.elements[i];
            if (
                this.elements[node.nextIndex].value.gt(value) &&
                node.value.lt(value)
            ) {
                return i;
            } else if (node.value.eq(value)) {
                throw new Error('Element already exists in the array');
            }
        }
        // If no such element was found, it means that our value is going to be the greatest in the array.
        // This means that the currently greatest element is going to be the low element of our value.
        return this.highestElementIndex;
    }

    /**
     * Returns the low element for the given value and the next value for that low element.
     * Low element is the greatest element which still has lower value than the provided one.
     * Low elements are used in non-membership proofs.
     */
    public findLowElement(
        value: BN,
    ): [IndexedElement | undefined, BN | undefined] {
        const lowElementIndex = this.findLowElementIndex(value);
        if (lowElementIndex === undefined) return [undefined, undefined];
        const lowElement = this.elements[lowElementIndex];
        return [lowElement, this.elements[lowElement.nextIndex].value];
    }

    // /**
    //  * Returns the index of the low element for the given `value`, which should be the part of the array.
    //  * Low element is the greatest element which still has lower value than the provided one.
    //  * Low elements are used in non-membership proofs.
    //  */
    // public findLowElementIndexForExistingElement(
    //     value: BN,
    // ): number | undefined {
    //     for (let i = 0; i <= this.length(); i++) {
    //         const node = this.elements[i];
    //         if (this.elements[node.nextIndex].value === value) {
    //             return i;
    //         }
    //     }
    //     return undefined;
    // }

    /**
     * Returns the hash of the given element. That hash consists of:
     * - The value of the given element.
     * - The `nextIndex` of the given element.
     * - The value of the element pointed by `nextIndex`.
     */
    public hashElement(
        lightWasm: LightWasm,
        index: number,
    ): Uint8Array | undefined {
        const element = this.elements[index];
        if (!element) return undefined;
        const nextElement = this.elements[element.nextIndex];
        if (!nextElement) return undefined;

        const hash = lightWasm.poseidonHash([
            bn(element.value.toArray('be', 32)).toString(),
            bn(element.nextIndex).toString(),
            bn(nextElement.value.toArray('be', 32)).toString(),
            // bigintToBeBytesArray<32>(element.value),
            // bigintToBeBytesArray<32>(element.nextIndex),
            // bigintToBeBytesArray<32>(nextElement.value),
        ]);
        // const hash = this.hasher.hashv([
        //     element.value.toBuffer(),
        //     new Uint8Array(new Uint32Array([element.nextIndex]).buffer),
        //     nextElement.value.toBuffer(),
        // ]);

        return hash;
    }

    /**
     * Appends a new element with the given value to the indexed array.
     * It finds the low element index and uses it to append the new element correctly.
     * @param value The value of the new element to append.
     * @returns The new element and its low element after insertion.
     */
    public append(value: BN): IndexedElementBundle {
        const lowElementIndex = this.findLowElementIndex(value);
        if (lowElementIndex === undefined) {
            throw new Error('Low element index not found.');
        }
        return this.appendWithLowElementIndex(lowElementIndex, value);
    }

    /**
     * Appends a new element with the given value to the indexed array using a specific low element index.
     * This method ensures the new element is placed correctly relative to the low element.
     * @param lowElementIndex The index of the low element.
     * @param value The value of the new element to append.
     * @returns The new element and its updated low element.
     */
    public appendWithLowElementIndex(
        lowElementIndex: number,
        value: BN,
    ): IndexedElementBundle {
        const lowElement = this.elements[lowElementIndex];

        if (lowElement.nextIndex === 0) {
            if (value.lte(lowElement.value)) {
                throw new Error(
                    'New element value must be greater than the low element value.',
                );
            }
        } else {
            const nextElement = this.elements[lowElement.nextIndex];

            if (value.lte(lowElement.value)) {
                throw new Error(
                    'New element value must be greater than the low element value.',
                );
            }

            if (value.gte(nextElement.value)) {
                throw new Error(
                    'New element value must be less than the next element value.',
                );
            }
        }

        const newElementBundle = this.newElementWithLowElementIndex(
            lowElementIndex,
            value,
        );

        // If the old low element wasn't pointing to any element, it means that:
        //
        // * It used to be the highest element.
        // * Our new element, which we are appending, is going the be the
        //   highest element.
        //
        // Therefore, we need to save the new element index as the highest
        // index.
        if (lowElement.nextIndex === 0) {
            this.highestElementIndex = newElementBundle.newElement.index;
        }

        // Insert new node.
        this.currentNodeIndex = newElementBundle.newElement.index;
        this.elements[this.length()] = newElementBundle.newElement;

        // Update low element.
        this.elements[lowElementIndex] = newElementBundle.newLowElement;

        return newElementBundle;
    }

    /**
     * Finds the lowest element in the array.
     * @returns The lowest element or undefined if the array is empty.
     */
    public lowest(): IndexedElement | undefined {
        return this.elements.length > 0 ? this.elements[0] : undefined;
    }

    /**
     * Creates a new element with the specified value and updates the low element index accordingly.
     * @param lowElementIndex The index of the low element.
     * @param value The value for the new element.
     * @returns A bundle containing the new element, the updated low element, and the value of the next element.
     */
    public newElementWithLowElementIndex(
        lowElementIndex: number,
        value: BN,
    ): IndexedElementBundle {
        const newLowElement = this.elements[lowElementIndex];

        const newElementIndex = this.currentNodeIndex + 1;
        const newElement = new IndexedElement(
            newElementIndex,
            value,
            newLowElement.nextIndex,
        );
        newLowElement.nextIndex = newElementIndex;

        const newElementNextValue = this.elements[newElement.nextIndex].value;

        return new IndexedElementBundle(
            newLowElement,
            newElement,
            newElementNextValue,
        );
    }

    /**
     * Creates a new element with the specified value by first finding the appropriate low element index.
     * @param value The value for the new element.
     * @returns A bundle containing the new element, the updated low element, and the value of the next element.
     */
    public newElement(value: BN): IndexedElementBundle {
        const lowElementIndex = this.findLowElementIndex(value);
        if (lowElementIndex === undefined) {
            throw new Error('Low element index not found.');
        }
        return this.newElementWithLowElementIndex(lowElementIndex, value);
    }
}

if (import.meta.vitest) {
    const { it, expect, describe } = import.meta.vitest;

    let WasmFactory: any;
    const refIndexedMerkleTreeInitedRoot = [
        14, 189, 9, 35, 134, 65, 9, 119, 107, 233, 168, 103, 222, 227, 207, 119,
        88, 137, 200, 189, 52, 117, 226, 207, 91, 63, 70, 253, 103, 91, 73, 117,
    ];

    const refIndexedMerkleTreeRootWithOneAppend = [
        26, 218, 104, 100, 17, 147, 90, 196, 182, 90, 15, 100, 24, 36, 207, 133,
        170, 188, 191, 9, 56, 50, 85, 155, 198, 213, 143, 67, 210, 228, 102,
        251,
    ];

    const refIndexedMerkleTreeRootWithTwoAppends = [
        45, 235, 123, 55, 121, 64, 246, 38, 138, 51, 143, 120, 23, 137, 129,
        116, 87, 55, 17, 251, 72, 171, 182, 91, 226, 15, 94, 53, 242, 140, 171,
        163,
    ];
    const refIndexedMerkleTreeRootWithThreeAppends = [
        9, 226, 16, 16, 135, 140, 213, 205, 247, 163, 245, 160, 135, 6, 12, 61,
        53, 32, 89, 69, 23, 22, 108, 242, 97, 209, 63, 239, 12, 20, 217, 155,
    ];

    const refNonInclusionProofAddress1 = {
        root: [
            26, 218, 104, 100, 17, 147, 90, 196, 182, 90, 15, 100, 24, 36, 207,
            133, 170, 188, 191, 9, 56, 50, 85, 155, 198, 213, 143, 67, 210, 228,
            102, 251,
        ],
        value: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 30,
        ],
        leaf_lower_range_value: [
            48, 100, 78, 114, 225, 49, 160, 41, 184, 80, 69, 182, 129, 129, 88,
            93, 40, 51, 232, 72, 121, 185, 112, 145, 67, 225, 245, 147, 240, 0,
            0, 0,
        ],
        leaf_higher_range_value: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        leaf_index: 1,
        next_index: 0,
        merkle_proof: [
            [
                26, 50, 171, 206, 227, 183, 227, 32, 10, 169, 16, 37, 85, 40,
                53, 64, 210, 75, 86, 126, 228, 12, 77, 212, 132, 165, 199, 160,
                92, 129, 206, 31,
            ],
            [
                15, 72, 55, 99, 47, 211, 74, 96, 126, 190, 2, 253, 166, 137,
                188, 165, 172, 110, 165, 253, 25, 166, 128, 195, 140, 105, 116,
                247, 87, 49, 214, 70,
            ],
            [
                16, 105, 103, 61, 205, 177, 34, 99, 223, 48, 26, 111, 245, 132,
                167, 236, 38, 26, 68, 203, 157, 198, 141, 240, 103, 164, 119,
                68, 96, 177, 241, 225,
            ],
            [
                24, 244, 51, 49, 83, 126, 226, 175, 46, 61, 117, 141, 80, 247,
                33, 6, 70, 124, 110, 234, 80, 55, 29, 213, 40, 213, 126, 178,
                184, 86, 210, 56,
            ],
            [
                7, 249, 216, 55, 203, 23, 176, 211, 99, 32, 255, 233, 59, 165,
                35, 69, 241, 183, 40, 87, 26, 86, 130, 101, 202, 172, 151, 85,
                157, 188, 149, 42,
            ],
            [
                43, 148, 207, 94, 135, 70, 179, 245, 201, 99, 31, 76, 93, 243,
                41, 7, 166, 153, 197, 140, 148, 178, 173, 77, 123, 92, 236, 22,
                57, 24, 63, 85,
            ],
            [
                45, 238, 147, 197, 166, 102, 69, 150, 70, 234, 125, 34, 204,
                169, 225, 188, 254, 215, 30, 105, 81, 185, 83, 97, 29, 17, 221,
                163, 46, 160, 157, 120,
            ],
            [
                7, 130, 149, 229, 162, 43, 132, 233, 130, 207, 96, 30, 182, 57,
                89, 123, 139, 5, 21, 168, 140, 181, 172, 127, 168, 164, 170,
                190, 60, 135, 52, 157,
            ],
            [
                47, 165, 229, 241, 143, 96, 39, 166, 80, 27, 236, 134, 69, 100,
                71, 42, 97, 107, 46, 39, 74, 65, 33, 26, 68, 76, 190, 58, 153,
                243, 204, 97,
            ],
            [
                14, 136, 67, 118, 208, 216, 253, 33, 236, 183, 128, 56, 158,
                148, 31, 102, 228, 94, 122, 204, 227, 226, 40, 171, 62, 33, 86,
                166, 20, 252, 215, 71,
            ],
            [
                27, 114, 1, 218, 114, 73, 79, 30, 40, 113, 122, 209, 165, 46,
                180, 105, 249, 88, 146, 249, 87, 113, 53, 51, 222, 97, 117, 229,
                218, 25, 10, 242,
            ],
            [
                31, 141, 136, 34, 114, 94, 54, 56, 82, 0, 192, 178, 1, 36, 152,
                25, 166, 230, 225, 228, 101, 8, 8, 181, 190, 188, 107, 250, 206,
                125, 118, 54,
            ],
            [
                44, 93, 130, 246, 108, 145, 75, 175, 185, 112, 21, 137, 186,
                140, 252, 251, 97, 98, 176, 161, 42, 207, 136, 168, 208, 135,
                154, 4, 113, 181, 248, 90,
            ],
            [
                20, 197, 65, 72, 160, 148, 11, 184, 32, 149, 127, 90, 223, 63,
                161, 19, 78, 245, 196, 170, 161, 19, 244, 100, 100, 88, 242,
                112, 224, 191, 191, 208,
            ],
            [
                25, 13, 51, 177, 47, 152, 111, 150, 30, 16, 192, 238, 68, 216,
                185, 175, 17, 190, 37, 88, 140, 173, 137, 212, 22, 17, 142, 75,
                244, 235, 232, 12,
            ],
            [
                34, 249, 138, 169, 206, 112, 65, 82, 172, 23, 53, 73, 20, 173,
                115, 237, 17, 103, 174, 101, 150, 175, 81, 10, 165, 179, 100,
                147, 37, 224, 108, 146,
            ],
            [
                42, 124, 124, 155, 108, 229, 136, 11, 159, 111, 34, 141, 114,
                191, 106, 87, 90, 82, 111, 41, 198, 110, 204, 238, 248, 183, 83,
                211, 139, 186, 115, 35,
            ],
            [
                46, 129, 134, 229, 88, 105, 142, 193, 198, 122, 249, 193, 77,
                70, 63, 252, 71, 0, 67, 201, 194, 152, 139, 149, 77, 117, 221,
                100, 63, 54, 185, 146,
            ],
            [
                15, 87, 197, 87, 30, 154, 78, 171, 73, 226, 200, 207, 5, 13,
                174, 148, 138, 239, 110, 173, 100, 115, 146, 39, 53, 70, 36,
                157, 28, 31, 241, 15,
            ],
            [
                24, 48, 238, 103, 181, 251, 85, 74, 213, 246, 61, 67, 136, 128,
                14, 28, 254, 120, 227, 16, 105, 125, 70, 228, 60, 156, 227, 97,
                52, 247, 44, 202,
            ],
            [
                33, 52, 231, 106, 197, 210, 26, 171, 24, 108, 43, 225, 221, 143,
                132, 238, 136, 10, 30, 70, 234, 247, 18, 249, 211, 113, 182,
                223, 34, 25, 31, 62,
            ],
            [
                25, 223, 144, 236, 132, 78, 188, 79, 254, 235, 216, 102, 243,
                56, 89, 176, 192, 81, 216, 201, 88, 238, 58, 168, 143, 143, 141,
                243, 219, 145, 165, 177,
            ],
            [
                24, 204, 162, 166, 107, 92, 7, 135, 152, 30, 105, 174, 253, 132,
                133, 45, 116, 175, 14, 147, 239, 73, 18, 180, 100, 140, 5, 247,
                34, 239, 229, 43,
            ],
            [
                35, 136, 144, 148, 21, 35, 13, 27, 77, 19, 4, 210, 213, 79, 71,
                58, 98, 131, 56, 242, 239, 173, 131, 250, 223, 5, 100, 69, 73,
                210, 83, 141,
            ],
            [
                39, 23, 31, 180, 169, 123, 108, 192, 233, 232, 245, 67, 181, 41,
                77, 232, 102, 162, 175, 44, 156, 141, 11, 29, 150, 230, 115,
                228, 82, 158, 213, 64,
            ],
            [
                47, 246, 101, 5, 64, 246, 41, 253, 87, 17, 160, 188, 116, 252,
                13, 40, 220, 178, 48, 185, 57, 37, 131, 229, 248, 213, 150, 150,
                221, 230, 174, 33,
            ],
        ],
    };

    const refIndexedArrayElem0 = new IndexedElement(0, bn(0), 2);
    const refIndexedArrayElem1 = new IndexedElement(
        1,
        bn(FIELD_SIZE_SUB_ONE),
        0,
    );
    const refIndexedArrayElem2 = new IndexedElement(2, bn(30), 1);

    describe('IndexedArray', () => {
        // it("init should match ref", () => {
        //     const indexedArray = new IndexedArray(refIndexedArrayElem0, refIndexedArrayElem1, refIndexedArrayElem2);
        //     expect(indexedArray.root).toEqual(refIndexedMerkleTreeInitedRoot);
        // });

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
            expect(nextValue2).toEqual(bn(FIELD_SIZE_SUB_ONE));
        });

        it('should appendWithLowElementIndex', () => {
            const indexedArray = new IndexedArray(
                [
                    new IndexedElement(0, bn(0), 1),
                    new IndexedElement(1, bn(FIELD_SIZE_SUB_ONE), 0),
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
                bn(FIELD_SIZE_SUB_ONE),
            );
        });

        it('should append', () => {
            const indexedArray = new IndexedArray(
                [
                    new IndexedElement(0, bn(0), 1),
                    new IndexedElement(1, bn(FIELD_SIZE_SUB_ONE), 0),
                ],
                1,
                1,
            );
            const newElement = indexedArray.append(bn(30));
            expect(newElement.newElement).toEqual(refIndexedArrayElem2);
            expect(newElement.newLowElement).toEqual(refIndexedArrayElem0);
            expect(newElement.newElementNextValue).toEqual(
                bn(FIELD_SIZE_SUB_ONE),
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
                bn(FIELD_SIZE_SUB_ONE),
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
            let refItems1 = new IndexedElement(1, bn(FIELD_SIZE_SUB_ONE), 0);
            let refItems2 = new IndexedElement(2, bn(30), 3);
            let refItems3 = new IndexedElement(3, bn(42), 1);

            const newElement2 = indexedArray.append(bn(42));

            expect(newElement2.newElement).toEqual(refItems3);
            expect(newElement2.newLowElement).toEqual(refItems2);
            expect(newElement2.newElementNextValue).toEqual(
                bn(FIELD_SIZE_SUB_ONE),
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
            refItems1 = new IndexedElement(1, bn(FIELD_SIZE_SUB_ONE), 0);
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
}
