import { ExtensionContext, window, commands, WebviewPanel } from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  NotificationType,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;
let renderedContent: string = "";

export function activate(context: ExtensionContext) {
	let serverPath = "..\\server\\target\\debug\\unimarkup_ls.exe";
  serverPath = context.asAbsolutePath(serverPath);

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
  };

  client = new LanguageClient(
    'Unimarkup-LSP',
    'Unimarkup LSP',
    serverOptions,
    clientOptions
  );

  client.start();

  let previewPanel: WebviewPanel;

  client.onReady().then(
    () => {
      const disposableSidePreview = commands.registerCommand('um.preview', async () => {
        previewPanel = await initPreview(context);
      });

      context.subscriptions.push(disposableSidePreview);
    }
  ).then(
    () => client.onNotification(new NotificationType<RenderedContent>('extension/renderedContent'), (data: RenderedContent) => {
      renderedContent = data.content;
      previewPanel.webview.html = renderedContent;
    })
  );
}

interface RenderedContent {
  id: string,
  content: string
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}


async function initPreview(context: ExtensionContext): Promise<WebviewPanel> {
  const panel = window.createWebviewPanel(
    'umPreviewer',
    'Unimarkup Preview',
    // Open the second column for preview inside editor
    2,
    {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: []
    }
  );

  panel.webview.html = renderedContent;

  return panel;
}
