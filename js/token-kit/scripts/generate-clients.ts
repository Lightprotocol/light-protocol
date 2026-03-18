/**
 * Generate TypeScript clients from the Light Token IDL using Codama.
 */

import { createFromRoot } from 'codama';
import { renderVisitor } from '@codama/renderers-js';
import { setInstructionAccountDefaultValuesVisitor } from '@codama/visitors';
import { publicKeyValueNode } from 'codama';
import path from 'path';
import { fileURLToPath } from 'url';

import {
    lightTokenIdl,
    LIGHT_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM,
} from '../src/idl.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Output directory for generated TypeScript
const typescriptOutputDir = path.resolve(
    __dirname,
    '../src/generated',
);

console.log('Creating Codama instance from Light Token IDL...');
const codama = createFromRoot(lightTokenIdl);

// Apply default account values for common accounts
console.log('Applying default account values...');
codama.update(
    setInstructionAccountDefaultValuesVisitor([
        {
            account: 'systemProgram',
            defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
        },
        {
            account: 'selfProgram',
            defaultValue: publicKeyValueNode(LIGHT_TOKEN_PROGRAM_ID),
        },
    ]),
);

// Generate TypeScript client
console.log(`Generating TypeScript client to ${typescriptOutputDir}...`);
codama.accept(
    renderVisitor(typescriptOutputDir, {
        formatCode: true,
        dependencyMap: {
            // Map codama codecs to @solana/codecs
            generatedPackage: '@lightprotocol/token-kit',
        },
    }),
);

console.log('Generation complete!');
