
import buildMimc7 from "./mimc7.js";

export default async function getHashes() {
    const mimc7 = await buildMimc7();
    return {
        hash0: function (left, right) {
            return mimc7.hash(left, right);
        },
        hash1: function(key, value) {
            return mimc7.multiHash([key, value], F.one);
        },
        F: mimc7.F
    }
}

