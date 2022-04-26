import { ExtensionContext, window, commands, WebviewPanel, Uri } from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  NotificationType,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;
const renderedContents = new Map();

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
      if (data !== undefined) {
        const contentUri = Uri.parse(data.id.toString());
        renderedContents.set(contentUri.fsPath, data.content);
        previewPanel.webview.html = renderedContents.get(contentUri.fsPath);
        previewPanel.title = getPreviewTitle(contentUri);
      }
    })
  );

  window.onDidChangeActiveTextEditor(
    (activeEditor) => {
      let content = renderedContents.get(activeEditor?.document.uri.fsPath);
      if (content !== undefined && previewPanel !== undefined) {
        previewPanel.webview.html = content;
        previewPanel.title = getPreviewTitle(window.activeTextEditor?.document.uri);
      }
    }
  );
}

interface RenderedContent {
  id: Uri,
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
  
  let content = renderedContents.get(window.activeTextEditor?.document.uri.fsPath);
  if (content === undefined) {
    content = "";
  }

  panel.webview.html = content;
  panel.title = getPreviewTitle(window.activeTextEditor?.document.uri);

  return panel;
}

function getPreviewTitle(uri: Uri | undefined): string {
  if (uri === undefined) {
    return "Unimarkup Preview";
  }
  return "[Preview] " + uri.path.split(`/`).pop();
}
