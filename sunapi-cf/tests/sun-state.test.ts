// Core tests for the /sun-state endpoint
import { describe, it, expect } from "vitest";
import { TEST_LOCATIONS, createTestUrl, validateJsonResponse, setupGlobalTestHooks, getMiniflareInstance } from "./setup";

// Setup global hooks
setupGlobalTestHooks();

describe("/sun-state endpoint tests", () => {

    // Test Case 1: Valid requests with multiple locations
    describe("Valid requests with variety of coordinates", () => {
        for (const [locationKey, location] of Object.entries(TEST_LOCATIONS)) {
            it(`should return valid response for ${location.name}`, async () => {
                const url = createTestUrl("/sun-state", {
                    lat: location.lat,
                    lon: location.lon,
                    tz: location.tz,
                });

                const response = await getMiniflareInstance().dispatchFetch(url);

                // Debug: log the response to see what's happening
                console.log(
                    `Response status for ${location.name}: ${response.status}`,
                );
                if (response.status !== 200) {
                    const errorText = await response.text();
                    console.log(`Error response: ${errorText}`);
                }

                // Validate HTTP status
                expect(response.status).toBe(200);

                // Validate response format
                const data = await validateJsonResponse(response, [
                    "sun_up",
                    "time",
                ]);

                // Validate data types
                expect(typeof data.sun_up).toBe("boolean");
                expect(typeof data.time).toBe("string");

                // Validate time format (ISO 8601 with timezone)
                expect(data.time).toMatch(
                    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?[+-]\d{2}:\d{2}\[.*\]$/,
                );
            });
        }
    });

    // Test Case 2: Missing required parameters
    describe("Missing required parameters", () => {
        it("should return 400 for missing latitude", async () => {
            const url = createTestUrl("/sun-state", {
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
                tz: TEST_LOCATIONS.SAN_FRANCISCO.tz,
            });

            const response = await getMiniflareInstance().dispatchFetch(url);
            expect(response.status).toBe(400);
        });

        it("should return 400 for missing longitude", async () => {
            const url = createTestUrl("/sun-state", {
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                tz: TEST_LOCATIONS.SAN_FRANCISCO.tz,
            });

            const response = await getMiniflareInstance().dispatchFetch(url);
            expect(response.status).toBe(400);
        });
    });

    // Test Case 3: Timezone handling
    describe("Timezone handling", () => {
        it("should use UTC timezone when tz parameter is omitted", async () => {
            const url = createTestUrl("/sun-state", {
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
                // No tz parameter - should default to UTC
            });

            const response = await getMiniflareInstance().dispatchFetch(url);
            expect(response.status).toBe(200);

            const data = await validateJsonResponse(response, [
                "sun_up",
                "time",
            ]);

            // Should show UTC timezone
            expect(data.time).toContain("[UTC]");
        });

        it("should handle specific timezone correctly", async () => {
            const url = createTestUrl("/sun-state", {
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
                tz: "America/Los_Angeles",
            });

            const response = await getMiniflareInstance().dispatchFetch(url);
            expect(response.status).toBe(200);

            const data = await validateJsonResponse(response, [
                "sun_up",
                "time",
            ]);

            // Should show America/Los_Angeles timezone
            expect(data.time).toContain("[America/Los_Angeles]");
        });
    });

    // Test Case 4: Response format validation
    describe("Response format validation", () => {
        it("should return proper JSON content-type header", async () => {
            const url = createTestUrl("/sun-state", {
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
                tz: TEST_LOCATIONS.SAN_FRANCISCO.tz,
            });

            const response = await getMiniflareInstance().dispatchFetch(url);
            expect(response.headers.get("content-type")).toContain(
                "application/json",
            );
        });

        it("should have all required fields in response", async () => {
            const url = createTestUrl("/sun-state", {
                lat: TEST_LOCATIONS.TOKYO.lat,
                lon: TEST_LOCATIONS.TOKYO.lon,
                tz: TEST_LOCATIONS.TOKYO.tz,
            });

            const response = await getMiniflareInstance().dispatchFetch(url);
            const data = (await response.json()) as {
                sun_up: boolean;
                time: string;
            };

            // Check all required fields are present
            expect(data).toHaveProperty("sun_up");
            expect(data).toHaveProperty("time");

            // Validate field types
            expect(typeof data.sun_up).toBe("boolean");
            expect(typeof data.time).toBe("string");
        });
    });
});
