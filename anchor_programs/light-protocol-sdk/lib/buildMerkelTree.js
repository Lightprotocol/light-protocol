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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.buildMerkelTree = void 0;
const constants_1 = require("./constants");
const merkelTree_1 = __importDefault(require("./merkelTree"));
const toFixedHex_1 = require("./utils/toFixedHex");
const { U64 } = require('n64');
const buildMerkelTree = function (poseidonHash) {
    return __awaiter(this, void 0, void 0, function* () {
        // const programPubKey = constants_1.PROGRAM_ID;
        // // Fetch all the accounts owned by the specified program id
        // const leave_accounts = yield connection.getProgramAccounts(programPubKey, {
        //     filters: [{ dataSize: 106 + 222 }],
        // });
        // // console.log("leave_accounts: ", leave_accounts)
        // let leaves_to_sort = [];
        // /// Slices some data from the leaves
        // leave_accounts.map((acc) => {
        //   // skip leaves which are not inserted yet
        //   // console.log("tree bytes: ", Array.prototype.slice.call(tree).toString())
        //   // console.log("acc.account.data.slice(74, 106): ", Array.prototype.slice.call(acc.account.data.slice(74, 106)).toString())
        //   // console.log("tree == acc.account.data.slice(74, 106): ", Array.prototype.slice.call(tree).toString() == Array.prototype.slice.call(acc.account.data.slice(74, 106)).toString())
        //   if(acc.account.data[1] == 4 && Array.prototype.slice.call(tree).toString() == Array.prototype.slice.call(acc.account.data.slice(74, 106)).toString()){
        //     leaves_to_sort.push({
        //         index: U64(acc.account.data.slice(2, 10)).toString(),
        //         leaves: acc.account.data.slice(10, 74),
        //     });
        //   }
        // });
        // /// Sorts leaves and substracts float of index a by index b
        // leaves_to_sort.sort((a, b) => parseFloat(a.index) - parseFloat(b.index));
        // const leaves = [];
        // if (leaves_to_sort.length > 0) {
        //   /// Creates two leaves for each of the sorted leaves by slicing different parts
        //   for (let i = 0; i < leaves_to_sort.length; i++) {
        //       leaves.push((0, toFixedHex_1.toFixedHex)(leaves_to_sort[i].leaves.slice(0, 32).reverse()));
        //       leaves.push((0, toFixedHex_1.toFixedHex)(leaves_to_sort[i].leaves.slice(32, 64).reverse()));
        //   }
        // }
        return new merkelTree_1.default(constants_1.MERKLE_TREE_HEIGHT, poseidonHash);
    });
};
exports.buildMerkelTree = buildMerkelTree;
