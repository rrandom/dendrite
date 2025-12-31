import * as path from 'path';
import { workspace, ExtensionContext, window, commands } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';
import { DendriteTreeDataProvider } from './treeDataProvider';

let client: LanguageClient;
let treeDataProvider: DendriteTreeDataProvider | undefined;

export function activate(context: ExtensionContext) {
    // Use binary from clients/vscode/server directory
    const serverPath = context.asAbsolutePath(
        path.join('server', process.platform === 'win32' ? 'dendrite-server.exe' : 'dendrite-server')
    );

    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            transport: TransportKind.stdio,
            options: {
                env: {
                    ...process.env,
                    "RUST_LOG": "info"
                }
            }
        },
        debug: {
            command: serverPath,
            transport: TransportKind.stdio,
            options: {
                env: {
                    ...process.env,
                    "RUST_LOG": "debug"
                }
            }
        }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'markdown' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
        },
        outputChannelName: 'Dendrite Server'
    };

    client = new LanguageClient(
        'dendriteServer',
        'Dendrite Server',
        serverOptions,
        clientOptions
    );

    // Start client and register TreeView after it's ready
    client.start().then(() => {
        // Register TreeView
        treeDataProvider = new DendriteTreeDataProvider(client);
        const treeView = window.createTreeView('dendriteHierarchy', {
            treeDataProvider: treeDataProvider,
            showCollapseAll: true
        });

        // Register refresh command
        const refreshCommand = commands.registerCommand('dendrite.refreshHierarchy', () => {
            treeDataProvider?.refresh();
        });

        context.subscriptions.push(treeView, refreshCommand);
    }).catch((error) => {
        window.showErrorMessage(`Failed to start Dendrite server: ${error}`);
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

