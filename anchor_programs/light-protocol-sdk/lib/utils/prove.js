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
const prove = function (input) {
    return __awaiter(this, void 0, void 0, function* () {
        let wtns
        let keyBasePath
        console.log("input.inputNullifier.length ", input.inputNullifier.length);
        if (input.inputNullifier.length == 2) {
          keyBasePath = `./Light_circuits/build/circuits/transactionMasp2`
          let path = "./Light_circuits/build/circuits/transactionMasp2_js/transactionMasp2";
          const buffer = readFileSync(`${path}.wasm`);

          let witnessCalculator =  yield calculateWtns(buffer)
          console.time('Proof generation');

          wtns= yield witnessCalculator.calculateWTNSBin(stringifyBigInts(input),0);

        } else {
          keyBasePath = `./Light_circuits/build/circuits/transactionMasp10`

          let path = "./Light_circuits/build/circuits/transactionMasp10_js/transactionMasp10";
          const buffer = readFileSync(`${path}.wasm`);

          let witnessCalculator =  yield calculateWtns(buffer)
          console.time('Proof generation');

          wtns= yield witnessCalculator.calculateWTNSBin(stringifyBigInts(input),0);
        }
        console.log("witness calc success");
        console.log("keyBasePath ", keyBasePath);
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
        // console.log("proof.pi_a ", proof.pi_a);
        // const curve = yield ffjavascript.getCurveFromName(vKey.curve);
        // let neg_proof_a = curve.G1.neg(curve.G1.fromObject(proof.pi_a))
        // proof.pi_a = [
        //   ffjavascript.utils.stringifyBigInts(neg_proof_a.slice(0,32)).toString(),
        //     ffjavascript.utils.stringifyBigInts(neg_proof_a.slice(32,64)).toString(),
        //       '1'
        // ]
        // console.log("negated proof a");
        // console.log("proof.pi_a ", proof.pi_a);

        // console.log(neg_proof_a);
        // process.exit()
        // console.log("proof: ", proof);
        // we need to negate proof_a such that the product of pairings result is 1
        // doesnt work
        // proof.pi_a[1] = ffjavascript.utils.stringifyBigInts(curve.G1.F.toObject(curve.G1.F.neg(curve.G1.F.fromObject(proof.pi_a[1]))))
        // proof.pi_a = ffjavascript.utils.stringifyBigInts(curve.G1.toObject(curve.G1.neg(curve.G1.fromObject(proof.pi_a))));
        // console.log("proof: ", proof);

        // proof.pi_a = curve.G1.fromObject(proof.pi_a);
        // proof.pi_b = curve.G2.fromObject(proof.pi_b);
        // proof.pi_c = curve.G1.fromObject(proof.pi_c);
        //
        // console.log("proof: ", proof);
        // let proofBytes = Array.from(proof.pi_a).concat(Array.from(proof.pi_b).concat(Array.from(proof.pi_c)))
        // console.log("proofBytes: ", proofBytes);
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(0,32)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(32,64)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(64,96)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(96,128)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(128,160)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(128,160)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(160,192)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(192,224)), 32).toString());
        // console.log(ffjavascript.utils.leBuff2int(Buffer.from(proofBytes.slice(224,256)), 32).toString());

        const proofJson = JSON.stringify(proof, null, 1);
        return { proofJson, publicInputsJson };
    });
};
exports.prove = prove;
