// Test setup and utilities for sunapi-cf tests
import { Miniflare } from "miniflare";
import { beforeAll, afterAll, expect } from "vitest";

// Test locations with variety of coordinates
interface TestLocation {
    name: string;
    lat: number;
    lon: number;
    tz: string;
}

export const TEST_LOCATIONS: Record<string, TestLocation> = {
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

// Miniflare instance setup
let mf: Miniflare | undefined;

export function getMiniflareInstance(): Miniflare {
    if (!mf) {
        throw new Error("Miniflare instance not initialized");
    }
    return mf;
}

async function setupMiniflare() {
    mf = new Miniflare({
        scriptPath: "./build/index.js",
        modules: true,
        modulesRules: [
            { type: "CompiledWasm", include: ["**/*.wasm"], fallthrough: true },
        ],
        bindings: {},
        durableObjects: {
            SCHEDULED_WEBHOOK: "ScheduledWebhook",
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
export function validateJsonResponse(response: unknown, expectedFields: string[]) {
    expect((response as { headers: { get: (key: string) => string } }).headers.get("content-type")).toContain("application/json");
    return (response as { json: () => Promise<unknown> }).json().then((data: unknown) => {
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
