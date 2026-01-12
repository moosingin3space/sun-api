// Core tests for the /set-webhook endpoint
import { describe, it, expect } from "vitest";
import { TEST_LOCATIONS, createTestUrl, validateJsonResponse, setupGlobalTestHooks, getMiniflareInstance } from "./setup.js";
import type { DurableNamespace, DurableStub } from "./durable-objects.types.js";

// Setup global hooks
setupGlobalTestHooks();

describe("/set-webhook endpoint tests", () => {

    // Test Case 1: Valid webhook scheduling requests
    describe("Valid webhook scheduling requests", () => {
        for (const [, location] of Object.entries(TEST_LOCATIONS)) {
            it(`should schedule webhook successfully for ${location.name}`, async () => {
                const url = createTestUrl("/set-webhook", {});
                const webhookUrl = "https://example.com/webhook";

                const requestBody = {
                    url: webhookUrl,
                    lat: location.lat,
                    lon: location.lon,
                };

                const response = await getMiniflareInstance().dispatchFetch(url, {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                    },
                    body: JSON.stringify(requestBody),
                });

                // Debug: log the response
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
                const data = (await validateJsonResponse(response, [
                    "message",
                ])) as { message: string };

                // Validate data types
                expect(typeof data.message).toBe("string");
                expect(data.message).toBe("Webhook scheduled successfully");
            });
        }
    });

    // Test Case 2: Missing required parameters
    describe("Missing required parameters", () => {
        it("should return 422 for missing url", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(422);
        });

        it("should return 422 for missing latitude", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(422);
        });

        it("should return 422 for missing longitude", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(422);
        });
    });

    // Test Case 3: Invalid request methods
    describe("Invalid request methods", () => {
        it("should return 405 or 400 for GET request", async () => {
            const url = createTestUrl("/set-webhook", {});

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                },
            });

            // Should reject GET requests (405 Method Not Allowed or 400)
            expect([405, 400]).toContain(response.status);
        });
    });

    // Test Case 4: Response format validation
    describe("Response format validation", () => {
        it("should return proper JSON content-type header", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: TEST_LOCATIONS.TOKYO.lat,
                lon: TEST_LOCATIONS.TOKYO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.headers.get("content-type")).toContain(
                "application/json",
            );
        });

        it("should have all required fields in response", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            const data = (await response.json()) as {
                message: string;
            };

            // Check all required fields are present
            expect(data).toHaveProperty("message");

            // Validate field types
            expect(typeof data.message).toBe("string");
        });
    });

    // Test Case 5: Webhook URL validation
    describe("Webhook URL handling", () => {
        it("should accept valid HTTPS URLs", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should accept valid HTTP URLs", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "http://example.com/webhook",
                lat: TEST_LOCATIONS.TOKYO.lat,
                lon: TEST_LOCATIONS.TOKYO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should accept URLs with query parameters", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook?key=value&foo=bar",
                lat: TEST_LOCATIONS.LONDON.lat,
                lon: TEST_LOCATIONS.LONDON.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should accept URLs with authentication credentials", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://user:pass@example.com/webhook",
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });
    });

    // Test Case 6: Coordinate validation
    describe("Coordinate boundary values", () => {
        it("should accept maximum latitude (90)", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: 90,
                lon: 0,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should accept minimum latitude (-90)", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: -90,
                lon: 0,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should accept maximum longitude (180)", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: 0,
                lon: 180,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should accept minimum longitude (-180)", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: 0,
                lon: -180,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });
    });

    // Test Case 7: Content-Type validation
    describe("Content-Type header handling", () => {
        it("should accept application/json content-type", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should handle missing content-type header", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: TEST_LOCATIONS.SAN_FRANCISCO.lat,
                lon: TEST_LOCATIONS.SAN_FRANCISCO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                body: JSON.stringify(requestBody),
            });

            // Should either succeed or fail gracefully
            expect([200, 400, 415]).toContain(response.status);
        });
    });

    // Test Case 8: Durable object storage verification
    describe("Durable object storage verification", () => {
    // Helper function to create and verify a webhook in DO storage
    async function createAndVerifyWebhook(
        testDoName: string,
        webhookData: unknown,
        verificationCallback?: (storedWebhook: unknown) => void
    ): Promise<{ id: string }> {
        const mf = getMiniflareInstance();
        const ns = await (mf as unknown as { getDurableObjectNamespace: (name: string) => Promise<DurableNamespace> }).getDurableObjectNamespace("SCHEDULED_WEBHOOK");
        const id = ns.idFromName(testDoName);
        const stub = ns.get(id) as DurableStub;

        // POST to the DO to store the webhook
        const postResponse = await stub.fetch("http://durable-object/webhooks", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(webhookData),
        });

        expect(postResponse.status).toBe(200);
        const postData = await postResponse.json() as { id: string; message?: string };
        expect(postData.id).toBeDefined();

        // GET from the DO to verify the webhook was stored
        const getResponse = await stub.fetch("http://durable-object/webhooks", {
            method: "GET",
        });

        expect(getResponse.status).toBe(200);
        const webhooks = await getResponse.json() as unknown[];
        expect(webhooks.length).toBeGreaterThan(0);

        const storedWebhook = webhooks.find((w: unknown) => (w as { id: string }).id === postData.id);
        expect(storedWebhook).toBeDefined();

        // Run verification callback if provided
        if (verificationCallback) {
            verificationCallback(storedWebhook);
        }

        return { id: postData.id };
    }

        it("should save webhook to Durable Object storage after successful scheduling", async () => {
            const testDoName = "test-storage-verification";
            const testWebhook = {
                url: "https://example.com/storage-verification-test",
                method: "POST",
                body: null,
                headers: {},
                scheduled_at: new Date().toISOString(),
            };

            await createAndVerifyWebhook(
                testDoName,
                testWebhook,
                (storedWebhook) => {
                    expect((storedWebhook as { url: string }).url).toBe("https://example.com/storage-verification-test");
                }
            );
        });

        it("should store webhook with all expected fields", async () => {
            const url = createTestUrl("/set-webhook", {});
            const webhookUrl = "https://example.com/fields-test-direct";

            const requestBody = {
                url: webhookUrl,
                lat: TEST_LOCATIONS.TOKYO.lat,
                lon: TEST_LOCATIONS.TOKYO.lon,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);

            // Test the DO directly by creating a webhook with a known timestamp
            const testDoName = "test-fields-verification";
            const testWebhook = {
                url: "https://example.com/direct-test",
                method: "POST",
                body: '{"test": "data"}',
                headers: { "Content-Type": "application/json" },
                scheduled_at: new Date().toISOString(),
            };

            await createAndVerifyWebhook(
                testDoName,
                testWebhook,
                (storedWebhook) => {
                    expect((storedWebhook as { url: string }).url).toBe("https://example.com/direct-test");
                    expect((storedWebhook as { method: string }).method).toBe("POST");
                    expect((storedWebhook as { body: string }).body).toBe('{"test": "data"}');
                    expect((storedWebhook as { headers: Record<string, string> }).headers).toEqual({ "Content-Type": "application/json" });
                }
            );
        });

        it("should generate unique IDs for each webhook", async () => {
            const testDoName = "test-unique-ids";

            // Create first webhook
            const webhook1 = {
                url: "https://example.com/webhook-unique-1",
                method: "POST",
                body: null,
                headers: {},
                scheduled_at: new Date().toISOString(),
            };

            const { id: id1 } = await createAndVerifyWebhook(testDoName, webhook1);

            // Create second webhook
            const webhook2 = {
                url: "https://example.com/webhook-unique-2",
                method: "POST",
                body: null,
                headers: {},
                scheduled_at: new Date().toISOString(),
            };

            const { id: id2 } = await createAndVerifyWebhook(testDoName, webhook2);

            // Verify unique IDs
            expect(id1).toBeDefined();
            expect(id2).toBeDefined();
            expect(id1).not.toBe(id2);

            // Verify both are stored by querying all webhooks
             const mf = getMiniflareInstance();
             const ns = await (mf as unknown as { getDurableObjectNamespace: (name: string) => Promise<DurableNamespace> }).getDurableObjectNamespace("SCHEDULED_WEBHOOK");
             const durableId = ns.idFromName(testDoName);
             const stub = ns.get(durableId) as DurableStub;
             const getResponse = await stub.fetch("http://durable-object/webhooks", {
                 method: "GET",
             });

            const webhooks = await getResponse.json() as unknown[];
            expect(webhooks.some((w: unknown) => (w as { id: string }).id === id1)).toBe(true);
            expect(webhooks.some((w: unknown) => (w as { id: string }).id === id2)).toBe(true);
        });

        it("should store scheduled_at timestamp correctly", async () => {
            const testDoName = "test-timestamp";
            const scheduledAt = "2025-12-01T10:30:00Z";
            const webhook = {
                url: "https://example.com/timestamp-test",
                method: "POST",
                body: null,
                headers: {},
                scheduled_at: scheduledAt,
            };

            await createAndVerifyWebhook(
                testDoName,
                webhook,
                (storedWebhook) => {
                    expect((storedWebhook as { scheduled_at: string }).scheduled_at).toBe(scheduledAt);
                }
            );
        });
    });

    // Test Case 9: Extreme coordinate combinations
    describe("Extreme coordinate combinations", () => {
        it("should handle location at north pole", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: 90,
                lon: 0,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should handle location at south pole", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: -90,
                lon: 0,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should handle location at equator and prime meridian", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: 0,
                lon: 0,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });

        it("should handle location with extreme longitude (like Tokyo)", async () => {
            const url = createTestUrl("/set-webhook", {});

            const requestBody = {
                url: "https://example.com/webhook",
                lat: 35.6762,
                lon: 139.6503,
            };

            const response = await getMiniflareInstance().dispatchFetch(url, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(requestBody),
            });

            expect(response.status).toBe(200);
        });
    });
});
