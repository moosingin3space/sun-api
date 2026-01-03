// Test setup and utilities for sunapi-cf tests
import { Miniflare } from "miniflare";
import { beforeAll, afterAll, expect } from "vitest";

// Test locations with variety of coordinates
export const TEST_LOCATIONS = {
    SAN_FRANCISCO: {
        name: "San Francisco",
        lat: 37.7749,
        lon: -122.4194,
        tz: "America/Los_Angeles",
    },
    LONDON: {
        name: "London",
        lat: 51.5074,
        lon: -0.1278,
        tz: "Europe/London",
    },
    TOKYO: {
        name: "Tokyo",
        lat: 35.6762,
        lon: 139.6503,
        tz: "Asia/Tokyo",
    },
    SYDNEY: {
        name: "Sydney",
        lat: -33.8688,
        lon: 151.2093,
        tz: "Australia/Sydney",
    },
};

// Fixed timestamp for deterministic testing
// Using: 2025-11-22T12:00:00Z (UTC)
export const FIXED_TIMESTAMP = "2025-11-22T12:00:00Z";

// Miniflare instance setup
let mf: Miniflare;

export function getMiniflareInstance() {
    return mf;
}

export async function setupMiniflare() {
    mf = new Miniflare({
        scriptPath: "./build/index.js",
        modules: true,
        modulesRules: [
            { type: "CompiledWasm", include: ["**/*.wasm"], fallthrough: true },
        ],
        // Set up bindings and environment for testing
        bindings: {
            // Add any required bindings here
        },
    });

    return mf;
}

// Utility function to create test URL
export function createTestUrl(
    path: string,
    params: Record<string, string | number> = {},
): string {
    const url = new URL(path, "http://localhost");
    Object.entries(params).forEach(([key, value]) => {
        url.searchParams.append(key, String(value));
    });
    return url.toString();
}

// Utility function to validate JSON response
export function validateJsonResponse(response: any, expectedFields: string[]) {
    expect(response.headers.get("content-type")).toContain("application/json");
    return response.json().then((data: any) => {
        expectedFields.forEach((field) => {
            expect(data).toHaveProperty(field);
        });
        return data;
    });
}

// Global setup and teardown
export function setupGlobalTestHooks() {
    beforeAll(async () => {
        mf = await setupMiniflare();
    });

    afterAll(async () => {
        await mf?.dispose();
    });
}
