import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import reader from "../src/pi-reader.mjs";

test("Pi keeps the FolderLens reader and exact-model boundary", async () => {
	const root = mkdtempSync(join(tmpdir(), "folderlens-pi-"));
	const before = { ...process.env };
	try {
		const executable = join(root, "reader");
		writeFileSync(
			executable,
			`#!/bin/sh
test "$1" = --folderlens-read && test "$2" = private-socket || exit 2
read -r input
printf '%s\\n' '{"text":"selected text"}'
`,
			{ mode: 0o700 },
		);
		process.env.FOLDERLENS_READER_EXECUTABLE = executable;
		process.env.FOLDERLENS_READER_SOCKET = "private-socket";
		process.env.FOLDERLENS_EXPECTED_MODEL = "provider/saved-model";
		delete process.env.SHELF_READER_EXECUTABLE;
		delete process.env.SHELF_READER_SOCKET;
		delete process.env.SHELF_EXPECTED_MODEL;
		const handlers = {};
		let tool;
		reader({
			on: (name, handler) => {
				handlers[name] = handler;
			},
			registerTool: (value) => {
				tool = value;
			},
		});
		assert.equal(tool.name, "folderlens_read_selected");
		assert.deepEqual(await tool.execute("read", { path: "/selected" }), {
			content: [{ type: "text", text: "selected text" }],
			details: {},
		});
		const ctx = {
			ui: { notify() {} },
			model: { provider: "provider", id: "other-model" },
		};
		assert.deepEqual(await handlers.input({}, ctx), { action: "handled" });
		ctx.model.id = "saved-model";
		assert.deepEqual(await handlers.input({}, ctx), { action: "continue" });
		assert.equal(await handlers.tool_call({ toolName: tool.name }), undefined);
		for (const toolName of ["read", "bash", "shelf_read_selected"]) {
			assert.equal((await handlers.tool_call({ toolName })).block, true);
		}
		assert.equal((await handlers.user_bash()).result.exitCode, 1);
		delete process.env.FOLDERLENS_READER_SOCKET;
		assert.throws(() => reader({}), /Launch this reader/);
	} finally {
		process.env = before;
		rmSync(root, { recursive: true, force: true });
	}
});
