import { describe, it, expect } from "vitest";
import { setupGlobalTestHooks, getMiniflareInstance } from "./setup";

setupGlobalTestHooks();

describe("Worker", () => {
    it("GET / returns 200", async () => {
        const response = await getMiniflareInstance().dispatchFetch("http://localhost/");
        expect(response.status).toBe(200);
    });
});
