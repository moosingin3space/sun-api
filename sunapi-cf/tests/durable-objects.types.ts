/**
 * Type definitions for Miniflare Durable Objects.
 *
 * These types are inferred from Miniflare's DurableObjectNamespace API since
 * @cloudflare/workers-types is not directly available in the test environment.
 */

/**
 * A namespace for accessing Durable Objects by ID or name.
 */
export interface DurableNamespace {
    /**
     * Create a Durable Object ID from a name string.
     */
    idFromName(name: string): unknown;
    /**
     * Get a stub for communicating with a specific Durable Object instance.
     */
    get(id: unknown): unknown;
}

/**
 * A stub for sending HTTP requests to a Durable Object instance.
 */
export interface DurableStub {
    /**
     * Send an HTTP request to the Durable Object.
     */
    fetch(url: string, opts: object): Promise<Response>;
}
