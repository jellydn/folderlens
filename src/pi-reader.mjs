import { spawnSync } from "node:child_process";

/** @param {import("@earendil-works/pi-coding-agent").ExtensionAPI} pi */
export default function shelfReader(pi) {
	const executable = process.env.SHELF_READER_EXECUTABLE;
	const socket = process.env.SHELF_READER_SOCKET;
	if (!executable || !socket)
		throw new Error("Launch this reader from its Shelf run.");

	pi.registerTool({
		name: "shelf_read_selected",
		label: "Shelf selected-folder reader",
		description:
			"Read bounded UTF-8 text or list a currently eligible selected directory. Requires Shelf running. No writes or shell commands.",
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
			const response = spawnSync(executable, ["--shelf-read", socket], {
				input: `${JSON.stringify(params)}\n`,
				encoding: "utf8",
				timeout: 5000,
				maxBuffer: 4194304,
			});
			if (response.error || response.status !== 0) {
				throw new Error(
					"Shelf reader is unavailable. Keep Shelf open for folder inspection.",
				);
			}
			const result = JSON.parse(response.stdout);
			if (result.error) throw new Error(result.error);
			return { content: [{ type: "text", text: result.text }], details: {} };
		},
	});
	// Interactive /tools cannot turn a native file or shell tool back into a grant.
	pi.on("tool_call", async (event) => {
		if (event.toolName !== "shelf_read_selected") {
			return {
				block: true,
				reason: "This Shelf chat permits only its selected-folder reader.",
			};
		}
	});
	pi.on("user_bash", async () => ({
		result: {
			output: "Shell commands are disabled in this Shelf review chat.",
			exitCode: 1,
			cancelled: false,
			truncated: false,
		},
	}));
}
