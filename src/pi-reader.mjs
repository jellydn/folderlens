import { spawnSync } from "node:child_process";

/** @param {import("@earendil-works/pi-coding-agent").ExtensionAPI} pi */
export default function folderlensReader(pi) {
	const executable = process.env.FOLDERLENS_READER_EXECUTABLE;
	const socket = process.env.FOLDERLENS_READER_SOCKET;
	if (!executable || !socket)
		throw new Error("Launch this reader from its FolderLens run.");

	// Pi can fuzzy-match a removed model when restoring a saved interactive chat.
	pi.on("input", async (_event, ctx) => {
		const expected = process.env.FOLDERLENS_EXPECTED_MODEL;
		if (
			expected &&
			(!ctx.model || `${ctx.model.provider}/${ctx.model.id}` !== expected)
		) {
			ctx.ui.notify(
				"FolderLens blocked this prompt: Pi did not restore the exact saved model. Choose that model or start a new FolderLens review; no paid fallback is allowed.",
				"error",
			);
			return { action: "handled" };
		}
		return { action: "continue" };
	});

	pi.registerTool({
		name: "folderlens_read_selected",
		label: "FolderLens selected-folder reader",
		description:
			"Read bounded UTF-8 text or list a currently eligible selected directory. Requires FolderLens running. No writes or shell commands.",
		parameters: {
			type: "object",
			properties: {
				path: { type: "string" },
				line: { type: "integer", minimum: 1 },
				limit: { type: "integer", minimum: 1, maximum: 2000 },
			},
			required: ["path"],
			additionalProperties: false,
		},
		async execute(_id, params) {
			const response = spawnSync(executable, ["--folderlens-read", socket], {
				input: `${JSON.stringify(params)}\n`,
				encoding: "utf8",
				timeout: 5000,
				maxBuffer: 4194304,
			});
			if (response.error || response.status !== 0) {
				throw new Error(
					"FolderLens reader is unavailable. Keep FolderLens open for folder inspection.",
				);
			}
			const result = JSON.parse(response.stdout);
			if (result.error) throw new Error(result.error);
			return { content: [{ type: "text", text: result.text }], details: {} };
		},
	});
	// Interactive /tools cannot turn a native file or shell tool back into a grant.
	pi.on("tool_call", async (event) => {
		if (event.toolName !== "folderlens_read_selected") {
			return {
				block: true,
				reason: "This FolderLens chat permits only its selected-folder reader.",
			};
		}
	});
	pi.on("user_bash", async () => ({
		result: {
			output: "Shell commands are disabled in this FolderLens review chat.",
			exitCode: 1,
			cancelled: false,
			truncated: false,
		},
	}));
}
