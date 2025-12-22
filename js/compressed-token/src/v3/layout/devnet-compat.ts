/**
 * @internal
 * Temporary devnet compatibility config.
 * TODO: Remove after devnet program update (deployed at slot 426761768, Dec 8, 2025).
 */

let _useDevnetFormat = false;

/**
 * Enable V1 instruction format for devnet compatibility.
 * Call this before any mint operations when targeting devnet.
 */
export function setDevnetCompat(enabled: boolean): void {
    _useDevnetFormat = enabled;
}

/**
 * Check if devnet compatibility mode is enabled.
 */
export function isDevnetCompat(): boolean {
    return _useDevnetFormat;
}

/**
 * Auto-detect and set devnet compat from RPC endpoint.
 */
export function setDevnetCompatFromEndpoint(endpoint: string): void {
    _useDevnetFormat = endpoint.toLowerCase().includes('devnet');
}
