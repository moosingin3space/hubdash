import { Miniflare } from "miniflare";
import { beforeAll, afterAll } from "vitest";

let mf: Miniflare;

export function getMiniflareInstance(): Miniflare {
    return mf;
}

async function setupMiniflare(): Promise<Miniflare> {
    mf = new Miniflare({
        workers: [
            {
                scriptPath: "./build/index.js",
                compatibilityDate: "2025-11-21",
                modules: true,
                modulesRules: [
                    { type: "CompiledWasm", include: ["**/*.wasm"], fallthrough: true },
                ],
                bindings: {
                    GITHUB_CLIENT_ID: "test-client-id",
                    GITHUB_CLIENT_SECRET: "test-client-secret",
                },
            },
        ],
    });
    return mf;
}

export function setupGlobalTestHooks() {
    beforeAll(async () => {
        mf = await setupMiniflare();
    });

    afterAll(async () => {
        await mf?.dispose();
    });
}
