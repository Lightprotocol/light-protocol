const ethers_1 = require("ethers");
export const toBuffer = (value, length) => Buffer.from(ethers_1.BigNumber.from(value)
    .toHexString()
    .slice(2)
    .padStart(length * 2, '0'), 'hex');
