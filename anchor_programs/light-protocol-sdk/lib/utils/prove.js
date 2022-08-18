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
const { readFileSync, writeFile } = require("fs");

Object.defineProperty(exports, "__esModule", { value: true });
exports.prove = void 0;
// const { groth16FullProve, zKey, wtns } = require('snarkjs');
const snarkjs = require('snarkjs');
const calculateWtns = require('../../../Light_circuits/build/circuits/transactionMasp2_js/witness_calculator.js')
const ffjavascript = require('ffjavascript');
const { stringifyBigInts } = ffjavascript.utils;
// const genProof = require('./Light_circuits/build/transactionMasp2_js/witness_calculator')
const prove = function (input, keyBasePath) {
    return __awaiter(this, void 0, void 0, function* () {
        let path = "./Light_circuits/build/circuits/transactionMasp2_js/transactionMasp2";
        const buffer = readFileSync(`${path}.wasm`);
        let wtns
        let witnessCalculator =  yield calculateWtns(buffer)
        console.time('Proof generation');

        wtns= yield witnessCalculator.calculateWTNSBin(stringifyBigInts(input),0);

        const { proof, publicSignals } = yield snarkjs.groth16.prove(`${keyBasePath}.zkey`, wtns);

        const publicInputsJson = JSON.stringify(publicSignals, null, 1);
        console.timeEnd('Proof generation');

        const vKey = yield snarkjs.zKey.exportVerificationKey(`${keyBasePath}.zkey`);
        const res = yield snarkjs.groth16.verify(vKey, publicSignals, proof);
        if (res === true) {
            console.log('Verification OK');
        }
        else {
            console.log('Invalid proof');
            throw new Error('Invalid Proof');
        }
        const proofJson = JSON.stringify(proof, null, 1);
        return { proofJson, publicInputsJson };
    });
};
exports.prove = prove;
