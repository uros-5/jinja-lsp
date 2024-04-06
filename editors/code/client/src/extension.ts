/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import * as path from 'path';
import { workspace, ExtensionContext, commands, window, Range, Selection } from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';
import * as child_process from 'child_process';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
	// The server is implemented in node
	const serverModule = getServer();
	if (!serverModule.valid) {
		throw new Error(serverModule.name);
	}

	let config: Record<string, any> = JSON.parse(
		JSON.stringify(workspace.getConfiguration("jinja-lsp"))
	);

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used
	const serverOptions: ServerOptions = {
		run: { command: serverModule.name },
		debug: {
			command: serverModule.name,
			args: [],
		}
	};

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'jinja-html' }, { scheme: 'file', language: 'rust' }, { scheme: 'file', language: 'python' }],
		initializationOptions: config,
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/.{jinja, rs, py}')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'jinja-lsp',
		'Jinja language server',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}

function getServer(): { valid: boolean, name: string } {
	try {
		// let name = "/home/uros/.cache/cargo/target/debug/jinja-lsp";
		let name = "jinja-lsp";
		const windows = process.platform === "win32";
		const suffix = windows ? ".exe" : "";
		const binaryName = name + suffix;
		const validation = child_process.spawnSync(name);
		if (validation.status === 0) {
			return { valid: true, name: binaryName };
		}
		else {
			return { valid: false, name: "Jinja language server not installed." }
		}

	}
	catch (e) {
		return { valid: false, name: "Jinja language server not installed." }
	}
}
