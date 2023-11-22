
import { blake2, blake2str } from './wasm/accountwasm';

// init(wasmUrl).then((instance) => {
//     console.log("accountwasm loaded", instance);
// });

export const isBoolean = (value: any) : value is boolean => typeof value === 'boolean';

export function blake(input: string|Uint8Array, hashLength: number = 32): Uint8Array {
    if (typeof input === 'string') {
        return blake2str(input, hashLength);
    }
    else {
        return blake2(input, hashLength);
    }    
}

