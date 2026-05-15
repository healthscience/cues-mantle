/* tslint:disable */
/* eslint-disable */

export class HeliCoord {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Calculates the "Network Arc" based on the Sun's position
     * relative to the Truth Longitude (-41.5).
     */
    get_network_arc(current_solar_longitude: number): number;
    /**
     * Instead of checking a "Date," the engine checks the
     * Earth's Ecliptic Longitude (L).
     * When L = 0.0, the Network Great Orbit begins.
     */
    is_network_equinox(ecliptic_longitude: number): boolean;
    constructor();
}

export class HeliCore {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * 1. CALCULATE ORBITAL POSITION (0-360°)
     * Replaces linear math with true elliptical ecliptic longitude.
     */
    static get_orbital_degree(timestamp_ms: bigint): number;
    /**
     * 2. CALCULATE ZENITH ANGLE (Degrees)
     * Used for the "Light Potential" and Local Solar Noon
     */
    static get_zenith_angle(lat: number, lon: number, timestamp_ms: bigint): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_helicoord_free: (a: number, b: number) => void;
    readonly __wbg_helicore_free: (a: number, b: number) => void;
    readonly helicoord_get_network_arc: (a: number, b: number) => number;
    readonly helicoord_is_network_equinox: (a: number, b: number) => number;
    readonly helicoord_new: () => number;
    readonly helicore_get_orbital_degree: (a: bigint) => number;
    readonly helicore_get_zenith_angle: (a: number, b: number, c: bigint) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
