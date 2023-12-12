import { Hasher } from "@lightprotocol/account.rs";
export const DEFAULT_ZERO =
  "14522046728041339886521211779101644712859239303505368468566383402165481390632";

/**
 * @callback hashFunction
 * @param left Left leaf
 * @param right Right leaf
 */
/**
 * Merkle tree
 */
export class MerkleTree {
  /**
   * Constructor
   * @param {number} levels Number of levels in the tree
   * @param {Array} [elements] Initial elements
   * @param {Object} options
   * @param {hashFunction} [options.hashFunction] Function used to hash 2 leaves
   * @param [options.zeroElement] Value for non-existent leaves
   */
  levels: number;
  capacity: number;
  zeroElement;
  _hasher: Hasher;
  _zeros: string[];
  _layers: string[][];

  constructor(
    levels: number,
    hasher: Hasher,
    elements: string[] = [],
    { zeroElement = DEFAULT_ZERO } = {},
  ) {
    this.levels = levels;
    this.capacity = 2 ** levels;
    this.zeroElement = zeroElement;
    this._hasher = hasher;
    if (elements.length > this.capacity) {
      throw new Error("Tree is full");
    }
    this._zeros = [];
    this._layers = [];
    this._layers[0] = elements;
    this._zeros[0] = this.zeroElement;

    for (let i = 1; i <= levels; i++) {
      this._zeros[i] = this._hasher.poseidonHashString([
        this._zeros[i - 1],
        this._zeros[i - 1],
      ]);
    }
    this._rebuild();
  }

  _rebuild() {
    for (let level = 1; level <= this.levels; level++) {
      this._layers[level] = [];
      for (let i = 0; i < Math.ceil(this._layers[level - 1].length / 2); i++) {
        this._layers[level][i] = this._hasher.poseidonHashString([
          this._layers[level - 1][i * 2],
          i * 2 + 1 < this._layers[level - 1].length
            ? this._layers[level - 1][i * 2 + 1]
            : this._zeros[level - 1],
        ]);
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

  insert(element: string) {
    if (this._layers[0].length >= this.capacity) {
      throw new Error("Tree is full");
    }
    this.update(this._layers[0].length, element);
  }

  /**
   * Insert multiple elements into the tree. Tree will be fully rebuilt during this operation.
   * @param {Array} elements Elements to insert
   */
  bulkInsert(elements: string[]) {
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
  update(index: number, element: string) {
    // index 0 and 1 and element is the commitment hash
    if (
      isNaN(Number(index)) ||
      index < 0 ||
      index > this._layers[0].length ||
      index >= this.capacity
    ) {
      throw new Error("Insert index out of bounds: " + index);
    }
    this._layers[0][index] = element;
    for (let level = 1; level <= this.levels; level++) {
      index >>= 1;
      this._layers[level][index] = this._hasher.poseidonHashString([
        this._layers[level - 1][index * 2],
        index * 2 + 1 < this._layers[level - 1].length
          ? this._layers[level - 1][index * 2 + 1]
          : this._zeros[level - 1],
      ]);
    }
  }

  /**
   * Get merkle path to a leaf
   * @param {number} index Leaf index to generate path for
   * @returns {{pathElements: number[], pathIndex: number[]}} An object containing adjacent elements and left-right index
   */
  path(index: number) {
    if (isNaN(Number(index)) || index < 0 || index >= this._layers[0].length) {
      throw new Error("Index out of bounds: " + index);
    }
    const pathElements: string[] = [];
    const pathIndices: number[] = [];
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
  indexOf(element: string, comparator: Function | null = null) {
    if (comparator) {
      return this._layers[0].findIndex((el: string) => comparator(element, el));
    } else {
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
  static deserialize(data: any, hashFunction: Function) {
    const instance = Object.assign(Object.create(this.prototype), data);
    instance._hash = hashFunction;
    instance.capacity = 2 ** instance.levels;
    instance.zeroElement = instance._zeros[0];
    return instance;
  }
}
