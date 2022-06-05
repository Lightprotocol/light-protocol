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
exports.prove = void 0;
const { groth16, zKey } = require('snarkjs');
const ffjavascript = require('ffjavascript');
const { stringifyBigInts } = ffjavascript.utils;
const prove = function (input, keyBasePath) {
    return __awaiter(this, void 0, void 0, function* () {
        console.time('Proof generation');
        const { proof, publicSignals } = yield groth16.fullProve(stringifyBigInts(input), `${keyBasePath}.wasm`, `${keyBasePath}.zkey`);
        const publicInputsJson = JSON.stringify(publicSignals, null, 1);
        console.timeEnd('Proof generation');
        const vKey = yield zKey.exportVerificationKey(`${keyBasePath}.zkey`);
        const res = yield groth16.verify(vKey, publicSignals, proof);
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
