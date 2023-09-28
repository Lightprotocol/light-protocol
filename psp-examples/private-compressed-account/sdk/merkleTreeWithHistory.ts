import { FIELD_SIZE } from "@lightprotocol/zk.js";

import { BN } from "@coral-xyz/anchor";

export class MerkleTreeWithHistory {
  levels: number;
  hasher: any;
  filledSubtrees: BN[] = [];
  root: BN;
  static readonly ROOT_HISTORY_SIZE = 100;
  currentRootIndex = 0;
  nextIndex = 0;
  leaves: BN[] = [];

  constructor(levels: number, hasher: any) {
    if (levels <= 0 || levels > 18) {
      throw new Error("Invalid levels");
    }
    this.levels = levels;
    this.hasher = hasher;

    this._initialize();
  }

  private _initialize() {
    for (let i = 0; i < this.levels; i++) {
      this.filledSubtrees[i] = new BN(zeroValues[i]);
    }
    this.root = new BN(zeroValues[this.levels]);
  }

  hashLeftRight(left: BN, right: BN): BN {
    if (left.gte(FIELD_SIZE) || right.gte(FIELD_SIZE)) {
      throw new Error("Values should be inside the field");
    }
    return new BN(this.hasher.F.toString(this.hasher([left, right])));
  }

  insert(leaf: BN): number {
    this.leaves.push(leaf);
    let _nextIndex = this.nextIndex;

    if (_nextIndex >= Math.pow(2, this.levels)) {
      throw new Error("Merkle tree is full. No more leaves can be added");
    }

    let currentIndex = _nextIndex;
    let currentLevelHash = leaf;
    let left: BN;
    let right: BN;

    for (let i = 0; i < this.levels; i++) {
      if (currentIndex % 2 === 0) {
        left = currentLevelHash;
        right = new BN(zeroValues[i]);
        this.filledSubtrees[i] = currentLevelHash;
      } else {
        left = this.filledSubtrees[i];
        right = currentLevelHash;
      }
      currentLevelHash = this.hashLeftRight(left, right);
      currentIndex = Math.floor(currentIndex / 2);
    }

    this.root = currentLevelHash;
    this.nextIndex = _nextIndex + 1;
    return _nextIndex;
  }
}

export const zeroValues: string[] = [
  "14522046728041339886521211779101644712859239303505368468566383402165481390632",
  "12399300409582020702502593817695692114365413884629119646752088755594619792099",
  "8395588225108361090185968542078819429341401311717556516132539162074718138649",
  "4057071915828907980454096850543815456027107468656377022048087951790606859731",
  "3743829818366380567407337724304774110038336483209304727156632173911629434824",
  "3362607757998999405075010522526038738464692355542244039606578632265293250219",
  "20015677184605935901566129770286979413240288709932102066659093803039610261051",
  "10225829025262222227965488453946459886073285580405166440845039886823254154094",
  "5686141661288164258066217031114275192545956158151639326748108608664284882706",
  "13358779464535584487091704300380764321480804571869571342660527049603988848871",
  "20788849673815300643597200320095485951460468959391698802255261673230371848899",
  "18755746780925592439082197927133359790105305834996978755923950077317381403267",
  "10861549147121384785495888967464291400837754556942768811917754795517438910238",
  "7537538922575546318235739307792157434585071385790082150452199061048979169447",
  "19170203992070410766412159884086833170469632707946611516547317398966021022253",
  "9623414539891033920851862231973763647444234218922568879041788217598068601671",
  "3060533073600086539557684568063736193011911125938770961176821146879145827363",
  "138878455357257924790066769656582592677416924479878379980482552822708744793",
  "15800883723037093133305280672853871715176051618981698111580373208012928757479",
];
