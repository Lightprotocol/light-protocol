import { LightWasm } from '../test-rpc/test-rpc';
import { BN } from '@coral-xyz/anchor';
import { bn } from '../../state';
import { MerkleTree } from './merkle-tree';
import { beforeAll } from 'vitest';
import { HIGHEST_ADDRESS_PLUS_ONE } from '../../constants';

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
        } catch (error) {
            throw new Error('Hashing failed');
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
            const init_value = HIGHEST_ADDRESS_PLUS_ONE;
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
        ]);

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
}
