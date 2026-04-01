import { readFile } from "fs/promises";
import { join } from "path";
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
                d1Databases: { DB: "test-sessions-db" },
            },
        ],
    });

    // Apply schema so the sessions table exists for any test that exercises auth.
    const migration = await readFile(
        join(__dirname, "../migrations/0001_sessions.sql"),
        "utf-8"
    );
    const db = await mf.getD1Database("DB");
    const statements = migration.split(";").map(s => s.trim()).filter(Boolean);
    for (const stmt of statements) {
        await db.prepare(stmt).run();
    }

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
