"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.timeoutPromise = void 0;
const timeoutPromise = function (seconds, promise) {
    // Create a promise that rejects in <ms> milliseconds
    const timeout = new Promise((resolve, reject) => {
        const id = setTimeout(() => {
            clearTimeout(id);
            reject('Timed out in ' + seconds + 's.');
        }, seconds * 1000);
    });
    // Returns a race between our timeout and the passed in promise
    return Promise.race([promise, timeout]);
};
exports.timeoutPromise = timeoutPromise;
