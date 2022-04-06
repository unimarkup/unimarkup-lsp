import * as path from 'path';
import { workspace, ExtensionContext, window } from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
	let serverPath = "..\\server\\target\\debug\\unimarkup_ls.exe";
  serverPath = context.asAbsolutePath(serverPath);

  // The debug options for the server
  // --inspect=6009: runs the server in Node's Inspector mode so VS Code can attach to the server for debugging
  //let debugOptions = { execArgv: ['--nolazy', '--inspect=6009'] };

  let serverOptions: ServerOptions = {
    run: { command: serverPath },
    debug: {
      command: serverPath
    }
  };

	const traceOutputChannel = window.createOutputChannel(
		'Unimarkup Language Server Trace',
	);

  let clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'unimarkup' }],
		traceOutputChannel
    // synchronize: {
    //   // Notify the server about file changes to '.clientrc files contained in the workspace
    //   fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
    // }
  };

  client = new LanguageClient(
    'Unimarkup-LSP',
    'Unimarkup LSP',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
