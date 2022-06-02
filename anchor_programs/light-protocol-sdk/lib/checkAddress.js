"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.checkAddress = void 0;
function checkAddress(addStr) {
    let match = addStr.match(/[1-9A-HJ-NP-Za-km-z]{32,44}/g); ///[1-9A-HJ-NP-Za-km-z]{32,44}/
    if (!match) {
        // throw new Error('The note has invalid format.')
        console.log('Address invalid');
        return false;
    }
    else {
        console.log('Address valid');
        return true;
    }
}
exports.checkAddress = checkAddress;
