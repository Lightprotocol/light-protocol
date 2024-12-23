import { LightWasm } from '../test-rpc/test-rpc';
import BN from 'bn.js';
import { bn } from '../../state';
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
