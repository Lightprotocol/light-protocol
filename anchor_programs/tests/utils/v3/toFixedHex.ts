const ethers_1 = require("ethers");
/** BigNumber to hex string of specified length */
export const toFixedHex = function (number, length = 32) {
    let result = '0x' +
        (number instanceof Buffer
            ? number.toString('hex')
            : ethers_1.BigNumber.from(number).toHexString().replace('0x', '')).padStart(length * 2, '0');
    if (result.indexOf('-') > -1) {
        result = '-' + result.replace('-', '');
    }
    return result;
};
