"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.buildMerkleTree = void 0;
const merkleTree_1 = require("./merkleTree");
const anchor = require("@project-serum/anchor");
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const MERKLE_TREE_HEIGHT = 18;
// TODO try to merge into merkle tree class
const buildMerkleTree = function ({ connection, config, merkleTreePubkey, merkleTreeProgram, poseidonHash }) {
    return __awaiter(this, void 0, void 0, function* () {
        let mtFetched = yield merkleTreeProgram.account.merkleTree.fetch(merkleTreePubkey);
        // Fetch all the accounts owned by the specified program id
        const leave_accounts = yield merkleTreeProgram.account.twoLeavesBytesPda.all();
        leave_accounts
            .sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());
        console.log(leave_accounts);
        const leaves = [];
        if (leave_accounts.length > 0) {
            for (let i = 0; i < leave_accounts.length; i++) {
                if (leave_accounts[i].account.leftLeafIndex.toNumber() < mtFetched.nextIndex.toNumber()) {
                    leaves.push(new anchor.BN(leave_accounts[i].account.nodeLeft, undefined, 'le').toString()); // .reverse()
                    leaves.push(new anchor.BN(leave_accounts[i].account.nodeRight, undefined, 'le').toString());
                }
            }
        }
        let fetchedMerkleTree = new merkleTree_1.MerkleTree(MERKLE_TREE_HEIGHT, poseidonHash, leaves);
        if (Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString() != mtFetched.roots[mtFetched.currentRootIndex].toString()) {
            throw `building merkle tree from chain failed: root local ${Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString()} != root fetched ${mtFetched.roots[mtFetched.currentRootIndex]}`;
        }
        return fetchedMerkleTree;
    });
};
exports.buildMerkleTree = buildMerkleTree;
