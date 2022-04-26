import { ExtensionContext, window, commands, WebviewPanel, Uri } from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  NotificationType,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;
const renderedContents = new Map<string, string>();

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

  let previewPanels = new Set<IdWebPanel>();
  let activePreviewPanel: IdWebPanel;
  let getActiveUriFsPath = () => {
    let uri = window.activeTextEditor?.document.uri;
    if (uri === undefined) {
      return "";
    } else {
      return uri.fsPath;
    }
  };

  client.onReady().then(
    () => {
      const disposableSidePreview = commands.registerCommand('um.preview', async () => {
        let uriFsPath = getActiveUriFsPath();
        let panelMatch = findFirstMatchingPanel(previewPanels, uriFsPath);

        if (previewPanels === undefined || previewPanels.size === 0
          || panelMatch === undefined || activePreviewPanel === undefined) {

          activePreviewPanel = await createPreview(context, previewPanels, uriFsPath);
        } else {
          activePreviewPanel = panelMatch;
          activePreviewPanel.panel.reveal(undefined, true);
        }
      });
      const disposableExplicitSidePreview = commands.registerCommand('um.explicitPreview', async () => {
        let uriFsPath = getActiveUriFsPath();
        activePreviewPanel = await createPreview(context, previewPanels, uriFsPath);
      });

      context.subscriptions.push(disposableSidePreview);
      context.subscriptions.push(disposableExplicitSidePreview);
    }
  ).then(
    () => client.onNotification(new NotificationType<RenderedContent>('extension/renderedContent'), (data: RenderedContent) => {
      if (data !== undefined) {
        const contentUri = Uri.parse(data.id.toString());
        renderedContents.set(contentUri.fsPath, data.content);

        let previewPanel = findFirstMatchingPanel(previewPanels, contentUri.fsPath);
        if (previewPanel !== undefined) {
          activePreviewPanel = previewPanel;
        } else if (activePreviewPanel !== undefined) {
          activePreviewPanel.id = contentUri.fsPath;
        }

        let content = renderedContents.get(contentUri.fsPath);
        if (content !== undefined && activePreviewPanel !== undefined) {
          activePreviewPanel.panel.webview.html = content;
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
          activePreviewPanel.panel.webview.html = content;
          activePreviewPanel.panel.title = getPreviewTitle(activeEditor?.document.uri);
          activePreviewPanel.panel.reveal(undefined, true);
        }
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

async function createPreview(context: ExtensionContext, panels: Set<IdWebPanel>, uriFsPath: string): Promise<IdWebPanel> {
  let content = renderedContents.get(uriFsPath);
  if (content === undefined) {
    content = "";
  }
  
  const panel = window.createWebviewPanel(
    'unimarkup.preview',
    'Unimarkup Preview',
    // Open the second column for preview inside editor
    2,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: []
    }
  );

  panel.webview.html = content;
  panel.title = getPreviewTitle(window.activeTextEditor?.document.uri);

  const idPanel = new IdWebPanel(uriFsPath, panel);
  panels.add(idPanel);

  idPanel.panel.onDidDispose(() => {
    panels.delete(idPanel);
  });

  return idPanel;
}

function getPreviewTitle(uri: Uri | undefined): string {
  if (uri === undefined) {
    return "Unimarkup Preview";
  }
  return "[Preview] " + uri.path.split(`/`).pop();
}
