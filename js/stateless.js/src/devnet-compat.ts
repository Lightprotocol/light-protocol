/**
 * @internal
 * Temporary devnet compatibility config.
 * TODO: Remove after devnet program update (deployed at slot 426761768, Dec 8, 2025).
 */

let _useDevnetFormat = false;

/**
 * @internal
 * Set devnet compat mode. Called automatically by createRpc().
 */
export function setDevnetCompat(enabled: boolean): void {
    _useDevnetFormat = enabled;
}

/**
 * Check if devnet compatibility mode is enabled.
 * Used by compressed-token SDK to select the correct instruction encoding.
 */
export function isDevnetCompat(): boolean {
    return _useDevnetFormat;
}

