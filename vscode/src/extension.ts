import path = require('path');
import { ExtensionContext, window, commands, WebviewPanel, Uri, ViewColumn, WebviewPanelSerializer, Webview, workspace } from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  NotificationType,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;
const renderedContents = new Map<string, string>();

const PANEL_VIEW_TYPE = 'unimarkup.preview';
const previewPanels = new Set<IdWebPanel>();
let activePreviewPanel: IdWebPanel;

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

  client.onReady().then(
    () => {
      const disposablePreview = commands.registerCommand('um.preview', async () => {
        activePreviewPanel = await createPreview(context, previewPanels, getActiveUriFsPath());
      });

      context.subscriptions.push(disposablePreview);
    }
  ).then(
    () => client.onNotification(new NotificationType<RenderedContent>('extension/renderedContent'), (data: RenderedContent) => {
      if (data !== undefined) {
        const contentUri = Uri.parse(data.id.toString());
        renderedContents.set(contentUri.fsPath, getHtmlTemplate(data.content));

        let previewPanel = findFirstMatchingPanel(previewPanels, contentUri.fsPath);
        if (previewPanel !== undefined) {
          activePreviewPanel = previewPanel;
        }

        let content = renderedContents.get(contentUri.fsPath);
        if (content !== undefined && activePreviewPanel !== undefined) {
          activePreviewPanel.id = contentUri.fsPath;
          activePreviewPanel.panel.webview.html = getWebviewContent(content, new PreviewState(activePreviewPanel.id));
          // activePreviewPanel.panel.webview.postMessage(new PreviewState(activePreviewPanel.id));
          activePreviewPanel.panel.title = getPreviewTitle(contentUri);
          activePreviewPanel.panel.reveal(undefined, true);
        }
      }
    })
  );

  window.onDidChangeActiveTextEditor(
    (activeEditor) => {
      let uriFsPath = activeEditor?.document.uri.fsPath;
      if (uriFsPath === undefined) {
        return;
      }

      if (activeEditor?.document.languageId === 'unimarkup') {
        let previewPanel = findFirstMatchingPanel(previewPanels, uriFsPath);
        if (previewPanel !== undefined) {
          activePreviewPanel = previewPanel;
        }

        let content = renderedContents.get(uriFsPath);
        if (content !== undefined && activePreviewPanel !== undefined) {
          activePreviewPanel.id = uriFsPath;
          activePreviewPanel.panel.webview.html = getWebviewContent(content, new PreviewState(activePreviewPanel.id));
          // activePreviewPanel.panel.webview.postMessage(new PreviewState(activePreviewPanel.id));
          activePreviewPanel.panel.title = getPreviewTitle(activeEditor?.document.uri);
          activePreviewPanel.panel.reveal(undefined, true);
        }
      }
    }
  );

  window.registerWebviewPanelSerializer(PANEL_VIEW_TYPE, new PreviewSerializer(context.extensionPath));
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

class IdWebPanel {
  id: string;
  panel: WebviewPanel;

  constructor(id: string, panel: WebviewPanel) {
    this.id = id;
    this.panel = panel;
  }
}

function findFirstMatchingPanel(panels: Set<IdWebPanel> | undefined, id: string): IdWebPanel | undefined {
  if (panels === undefined) {
    return undefined;
  }

  for (const panel of panels) {
    if (panel.id === id) {
      return panel;
    }
  }

  return undefined;
}

function getActiveUriFsPath(): string {
  let uri = window.activeTextEditor?.document.uri;
  if (uri === undefined) {
    return "";
  } else {
    return uri.fsPath;
  }
};

class PreviewSerializer implements WebviewPanelSerializer {
  extensionPath: string;

  constructor(extensionPath: string) {
    this.extensionPath = extensionPath;
  }

  async deserializeWebviewPanel(webviewPanel: WebviewPanel, state: any) {
    let uriFsPath = state ? state.id : undefined;
    if (uriFsPath !== undefined) {
      let content = renderedContents.get(uriFsPath);
      if (content === undefined) {
        const fs = require('fs');
        if (fs.existsSync(uriFsPath)) {
          workspace.openTextDocument(Uri.file(uriFsPath)); // Note: needed to start LSP rendering (somehow does not open the document though, which is convenient)
          content = getHtmlTemplate("<p>Loading...</p>");
        } else {
          content = getHtmlTemplate("<p>Original document does not exist anymore!</p>");
        }
      }
      webviewPanel.webview.html = getWebviewContent(content, new PreviewState(uriFsPath));

      let webPanel = new IdWebPanel(uriFsPath, webviewPanel);
      previewPanels.add(webPanel);

      webPanel.panel.onDidDispose(() => {
        previewPanels.delete(webPanel);
      });
    
      webPanel.panel.onDidChangeViewState((panelEvent) => {
        if (panelEvent.webviewPanel.active && webPanel !== activePreviewPanel) {
          activePreviewPanel = webPanel;
        }
      });
    }
  }
}

async function createPreview(context: ExtensionContext, panels: Set<IdWebPanel>, uriFsPath: string): Promise<IdWebPanel> {
  let content = renderedContents.get(uriFsPath);
  if (content === undefined) {
    content = getHtmlTemplate("<p>Loading</p>");
  }
  
  const panel = window.createWebviewPanel(
    PANEL_VIEW_TYPE,
    'Unimarkup Preview',
    ViewColumn.Two,
    {
      enableScripts: true,
    }
  );

  panel.webview.html = getWebviewContent(content, new PreviewState(uriFsPath));
  // panel.webview.postMessage(new PreviewState(uriFsPath));
  panel.title = getPreviewTitle(window.activeTextEditor?.document.uri);

  const idPanel = new IdWebPanel(uriFsPath, panel);
  panels.add(idPanel);

  idPanel.panel.onDidDispose(() => {
    panels.delete(idPanel);
  });

  idPanel.panel.onDidChangeViewState((panelEvent) => {
    if (panelEvent.webviewPanel.active && idPanel !== activePreviewPanel) {
      activePreviewPanel = idPanel;
    }
  });

  return idPanel;
}

function getPreviewTitle(uri: Uri | undefined): string {
  if (uri === undefined) {
    return "Unimarkup Preview";
  }
  return "[Preview] " + uri.path.split(`/`).pop();
}

class PreviewState {
  id: string;

  constructor(id: string) {
    this.id = id;
  }
}


function getWebviewContent(renderedPage: string, state: PreviewState): string {
  let stateScript = `
    <script type="text/javascript">
      const vscode = acquireVsCodeApi();
      vscode.setState(${JSON.stringify(state)});

      window.addEventListener('message', event => {
        vscode.setState(event.data);
      })
    </script>
  `;
  let headEnd = renderedPage.indexOf("</head>");

  return renderedPage.substring(0, headEnd) + stateScript + renderedPage.substring(headEnd);
}

function getHtmlTemplate(body: string): string {
  return `<!DOCTYPE html>
  <html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta http-equiv="Content-Security-Policy">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">

    <title>Preview</title>
  </head>
  <body>
    ${body}
  </body>
  </html>
  `;
}

