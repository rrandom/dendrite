import * as path from 'path';
import { workspace, ExtensionContext, window, ProgressLocation } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

import { registerTreeView } from './treeView';
import { registerRenameNoteCommand } from './commands/rename';
import { registerUndoCommand } from './commands/undo';
import { registerSplitNoteCommand } from './commands/split';
import { registerReorganizeHierarchyCommand } from './commands/hierarchy';
import { registerAuditCommand } from './commands/audit';
import { registerCreateNoteCommand } from './commands/createNote';


let client: LanguageClient;

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
            fileEvents: workspace.createFileSystemWatcher('**/*.md')
        },
        outputChannelName: 'Dendrite Server',
        middleware: {
            handleDiagnostics: (uri, diagnostics, next) => {
                console.log(`[Dendrite Client] Raw Diagnostics for ${uri.toString()}:`, JSON.stringify(diagnostics, null, 2));
                next(uri, diagnostics);
            }
        }
    };

    client = new LanguageClient(
        'dendriteServer',
        'Dendrite Server',
        serverOptions,
        clientOptions
    );

    // Start client and register TreeView and Commands after it's ready
    window.withProgress({
        location: ProgressLocation.Notification,
        title: "Dendrite",
        cancellable: false
    }, async (progress) => {
        progress.report({ message: "Starting..." });
        
        try {
            await client.start();
            
            // Register Tree View
            const treeDataProvider = registerTreeView(context, client);
    
            // Listen for hierarchy changes from server (e.g. external edits via git)
            client.onNotification('dendrite/hierarchyChanged', () => {
                console.log('[Dendrite Client] Received hierarchyChanged notification. Refreshing tree.');
                treeDataProvider.refresh();
            });
    
            // Register Commands
            context.subscriptions.push(
                registerRenameNoteCommand(client),
                registerUndoCommand(client),
                registerSplitNoteCommand(client),
                registerReorganizeHierarchyCommand(client),
                registerAuditCommand(client),
                registerCreateNoteCommand(client)
            );

            progress.report({ message: "Started" });
            // Keep the "Started" message visible briefly
            await new Promise(resolve => setTimeout(resolve, 1500));
        } catch (error) {
           window.showErrorMessage(`Failed to start Dendrite server: ${error}`);
        }
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
