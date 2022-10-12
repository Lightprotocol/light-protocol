const { readFileSync, writeFile } = require("fs");

const snarkjs = require('snarkjs');
const calculateWtns = require('../../../Light_circuits/build/circuits/transactionMasp2_js/witness_calculator.js')
const ffjavascript = require('ffjavascript');
const { stringifyBigInts } = ffjavascript.utils;
// const genProof = require('./Light_circuits/build/transactionMasp2_js/witness_calculator')
const prove = async function (input, keyBasePath) {
        let path = "./Light_circuits/build/circuits/transactionMasp2_js/transactionMasp2";
        const buffer = readFileSync(`${path}.wasm`);
        let wtns
        let witnessCalculator =  await calculateWtns(buffer)
        console.time('Proof generation');

        wtns= await witnessCalculator.calculateWTNSBin(stringifyBigInts(input),0);

        const { proof, publicSignals } = await snarkjs.groth16.prove(`${keyBasePath}.zkey`, wtns);

        const publicInputsJson = JSON.stringify(publicSignals, null, 1);
        console.timeEnd('Proof generation');

        const vKey = await snarkjs.zKey.exportVerificationKey(`${keyBasePath}.zkey`);
        const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
        if (res === true) {
            console.log('Verification OK');
        }
        else {
            console.log('Invalid proof');
            throw new Error('Invalid Proof');
        }
        const curve = await ffjavascript.getCurveFromName(vKey.curve);

        const proofJson = JSON.stringify(proof, null, 1);
        return { proofJson, publicInputsJson };
};
exports.prove = prove;
