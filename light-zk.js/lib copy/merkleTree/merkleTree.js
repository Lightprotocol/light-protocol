"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MerkleTree = void 0;
const index_1 = require("../index");
/**
 * @callback hashFunction
 * @param left Left leaf
 * @param right Right leaf
 */
/**
 * Merkle tree
 */
class MerkleTree {
    constructor(levels, poseidonHash2, elements = [], { zeroElement = index_1.DEFAULT_ZERO } = {}) {
        this.levels = levels;
        this.capacity = 2 ** levels;
        this.zeroElement = zeroElement;
        this._hash = poseidonHash2;
        if (elements.length > this.capacity) {
            throw new Error("Tree is full");
        }
        this._zeros = [];
        this._layers = [];
        this._layers[0] = elements;
        this._zeros[0] = this.zeroElement;
        for (let i = 1; i <= levels; i++) {
            this._zeros[i] = this._hash.F.toString(this._hash([this._zeros[i - 1], this._zeros[i - 1]]));
        }
        this._rebuild();
    }
    _rebuild() {
        for (let level = 1; level <= this.levels; level++) {
            this._layers[level] = [];
            for (let i = 0; i < Math.ceil(this._layers[level - 1].length / 2); i++) {
                this._layers[level][i] = this._hash.F.toString(this._hash([
                    this._layers[level - 1][i * 2],
                    i * 2 + 1 < this._layers[level - 1].length
                        ? this._layers[level - 1][i * 2 + 1]
                        : this._zeros[level - 1],
                ]));
            }
        }
    }
    /**
     * Get tree root
     * @returns {*}
     */
    root() {
        return this._layers[this.levels].length > 0
            ? this._layers[this.levels][0]
            : this._zeros[this.levels];
    }
    /**
     * Insert new element into the tree
     * @param element Element to insert
     */
    insert(element) {
        if (this._layers[0].length >= this.capacity) {
            throw new Error("Tree is full");
        }
        this.update(this._layers[0].length, element);
    }
    /**
     * Insert multiple elements into the tree. Tree will be fully rebuilt during this operation.
     * @param {Array} elements Elements to insert
     */
    bulkInsert(elements) {
        if (this._layers[0].length + elements.length > this.capacity) {
            throw new Error("Tree is full");
        }
        this._layers[0].push(...elements);
        this._rebuild();
    }
    // TODO: update does not work debug
    /**
     * Change an element in the tree
     * @param {number} index Index of element to change
     * @param element Updated element value
     */
    update(index, element) {
        // index 0 and 1 and element is the commitment hash
        if (isNaN(Number(index)) ||
            index < 0 ||
            index > this._layers[0].length ||
            index >= this.capacity) {
            throw new Error("Insert index out of bounds: " + index);
        }
        this._layers[0][index] = element;
        for (let level = 1; level <= this.levels; level++) {
            index >>= 1;
            this._layers[level][index] = this._hash(this._layers[level - 1][index * 2], index * 2 + 1 < this._layers[level - 1].length
                ? this._layers[level - 1][index * 2 + 1]
                : this._zeros[level - 1]);
        }
    }
    /**
     * Get merkle path to a leaf
     * @param {number} index Leaf index to generate path for
     * @returns {{pathElements: number[], pathIndex: number[]}} An object containing adjacent elements and left-right index
     */
    path(index) {
        if (isNaN(Number(index)) || index < 0 || index >= this._layers[0].length) {
            throw new Error("Index out of bounds: " + index);
        }
        const pathElements = [];
        const pathIndices = [];
        for (let level = 0; level < this.levels; level++) {
            pathIndices[level] = index % 2;
            pathElements[level] =
                (index ^ 1) < this._layers[level].length
                    ? this._layers[level][index ^ 1]
                    : this._zeros[level];
            index >>= 1;
        }
        return {
            pathElements,
            pathIndices,
        };
    }
    /**
     * Find an element in the tree
     * @param element An element to find
     * @param comparator A function that checks leaf value equality
     * @returns {number} Index if element is found, otherwise -1
     */
    indexOf(element, comparator = null) {
        if (comparator) {
            return this._layers[0].findIndex((el) => comparator(element, el));
        }
        else {
            return this._layers[0].indexOf(element);
        }
    }
    /**
     * Returns a copy of non-zero tree elements
     * @returns {Object[]}
     */
    elements() {
        return this._layers[0].slice();
    }
    /**
     * Serialize entire tree state including intermediate layers into a plain object
     * Deserializing it back will not require to recompute any hashes
     * Elements are not converted to a plain type, this is responsibility of the caller
     */
    serialize() {
        return {
            levels: this.levels,
            _zeros: this._zeros,
            _layers: this._layers,
        };
    }
    /**
     * Deserialize data into a MerkleTree instance
     * Make sure to provide the same hashFunction as was used in the source tree,
     * otherwise the tree state will be invalid
     *
     * @param data
     * @param hashFunction
     * @returns {MerkleTree}
     */
    static deserialize(data, hashFunction) {
        const instance = Object.assign(Object.create(this.prototype), data);
        instance._hash = hashFunction;
        instance.capacity = 2 ** instance.levels;
        instance.zeroElement = instance._zeros[0];
        return instance;
    }
}
exports.MerkleTree = MerkleTree;
//# sourceMappingURL=merkleTree.js.map