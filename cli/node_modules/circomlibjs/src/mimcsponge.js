import { Scalar, getCurveFromName } from "ffjavascript";
import { ethers } from "ethers";

const SEED = "mimcsponge";
const NROUNDS = 220;

export default async function buildMimcSponge() {
    const bn128 = await getCurveFromName("bn128", true);
    return new MimcSponge(bn128.Fr);
}

class MimcSponge {
    constructor (F) {
        this.F = F;
        this.cts = this.getConstants(SEED, NROUNDS);
    }

    getIV (seed)  {
        const F = this.F;
        if (typeof seed === "undefined") seed = SEED;
        const c = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(seed+"_iv"));
        const cn = Scalar.e(c);
        const iv = cn.mod(F.p);
        return iv;
    };

    getConstants (seed, nRounds)  {
        const F = this.F;
        if (typeof seed === "undefined") seed = SEED;
        if (typeof nRounds === "undefined") nRounds = NROUNDS;
        const cts = new Array(nRounds);
        let c = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(SEED));;
        for (let i=1; i<nRounds; i++) {
            c = ethers.utils.keccak256(c);

            cts[i] = F.e(c);
        }
        cts[0] = F.e(0);
        cts[cts.length - 1] = F.e(0);
        return cts;
    };


    hash(_xL_in, _xR_in, _k) {
        const F = this.F;
        let xL = F.e(_xL_in);
        let xR = F.e(_xR_in);
        const k = F.e(_k);
        for (let i=0; i<NROUNDS; i++) {
            const c = this.cts[i];
            const t = (i==0) ? F.add(xL, k) : F.add(F.add(xL, k), c);
            const t2 = F.square(t);
            const t4 = F.square(t2);
            const t5 = F.mul(t4, t);
            const xR_tmp = F.e(xR);
            if (i < (NROUNDS - 1)) {
                xR = xL;
                xL = F.add(xR_tmp, t5);
            } else {
                xR = F.add(xR_tmp, t5);
            }
        }
        return {
            xL: xL,
            xR: xR
        };
    }

    multiHash(arr, key, numOutputs)  {
        const F = this.F;
        if (typeof(numOutputs) === "undefined") {
            numOutputs = 1;
        }
        if (typeof(key) === "undefined") {
            key = F.zero;
        }

        let R = F.zero;
        let C = F.zero;

        for (let i=0; i<arr.length; i++) {
            R = F.add(R, F.e(arr[i]));
            const S = this.hash(R, C, key);
            R = S.xL;
            C = S.xR;
        }
        let outputs = [R];
        for (let i=1; i < numOutputs; i++) {
            const S = this.hash(R, C, key);
            R = S.xL;
            C = S.xR;
            outputs.push(R);
        }
        if (numOutputs == 1) {
            return outputs[0];
        } else {
            return outputs;
        }
    }
}
