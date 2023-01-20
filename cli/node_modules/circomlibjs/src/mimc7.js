import {getCurveFromName, Scalar} from "ffjavascript";

import { ethers } from "ethers";

const SEED = "mimc";
const NROUNDS = 91;

export default async function buildMimc7() {
    const bn128 = await getCurveFromName("bn128", true);
    return new Mimc7(bn128.Fr);
}


class Mimc7 {
    constructor (F) {
        this.F = F;
        this.cts = this.getConstants(SEED, 91);
    }

    getIV(seed) {
        const F = this.F;
        if (typeof seed === "undefined") seed = SEED;
        const c = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(seed+"_iv"));
        const cn = Scalar.e(c);
        const iv = Scalar.mod(cn, F.p);
        return iv;
    };

    getConstants(seed, nRounds) {
        const F = this.F;
        if (typeof seed === "undefined") seed = SEED;
        if (typeof nRounds === "undefined") nRounds = NROUNDS;
        const cts = new Array(nRounds);
        let c = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(SEED));
        for (let i=1; i<nRounds; i++) {
            c = ethers.utils.keccak256(c);

            cts[i] = F.e(c);
        }
        cts[0] = F.e(0);
        return cts;
    }

    hash (_x_in, _k) {
        const F = this.F;
        const x_in = F.e(_x_in);
        const k = F.e(_k);
        let r;
        for (let i=0; i<NROUNDS; i++) {
            const c = this.cts[i];
            const t = (i==0) ? F.add(x_in, k) : F.add(F.add(r, k), c);
            const t2 = F.square(t);
            const t4 = F.square(t2);
            r = F.mul(F.mul(t4, t2), t);
        }
        return F.add(r, k);
    }

    multiHash(arr, key) {
        const F = this.F;
        let r;
        if (typeof(key) === "undefined") {
            r = F.zero;
        } else {
            r = F.e(key);
        }
        for (let i=0; i<arr.length; i++) {
            r = F.add(
                F.add(
                    r,
                    F.e(arr[i])
                ),
                this.hash(F.e(arr[i]), r)
            );
        }
        return r;
    }
}
