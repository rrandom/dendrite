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

        const renameCommand = commands.registerCommand('dendrite.renameNote', async () => {
            const editor = window.activeTextEditor;
            if (!editor) {
                return;
            }

            const uri = editor.document.uri;
            try {
                // 1. Get current Note Key from server
                const result = await client.sendRequest<{ key: string }>('workspace/executeCommand', {
                    command: 'dendrite/getNoteKey',
                    arguments: [{ uri: uri.toString() }]
                });

                if (!result || !result.key) {
                    window.showErrorMessage('Failed to resolve Note Key for the current file.');
                    return;
                }

                // 2. Show input box pre-filled with the old key
                const newKey = await window.showInputBox({
                    title: 'Rename Note',
                    value: result.key,
                    prompt: 'Enter the new note identifier (NoteKey)',
                    placeHolder: 'e.g. projects.dendrite',
                    validateInput: (value) => {
                        if (!value || value.trim().length === 0) {
                            return 'Note name cannot be empty';
                        }
                        if (value === result.key) {
                            return 'New name must be different from the old name';
                        }
                        return null;
                    }
                });

                if (newKey) {
                    // 3. Trigger standard LSP rename flow using the new key
                    // This will invoke standard `textDocument/rename` providing the new name
                    await commands.executeCommand('vscode.executeDocumentRenameProvider', uri, editor.selection.active, newKey)
                        .then(async (edit) => {
                            if (edit) {
                                await workspace.applyEdit(edit as any); // Cast to any to avoid type issues with WorkspaceEdit
                            } else {
                                window.showErrorMessage('Rename provider failed to generate edits.');
                            }
                        });
                }
            } catch (error) {
                window.showErrorMessage(`Rename failed: ${error}`);
            }
        });

        const undoCommand = commands.registerCommand('dendrite.undoRefactor', async () => {
            try {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/undoRefactor',
                    arguments: []
                });
            } catch (error) {
                window.showErrorMessage(`Undo failed: ${error}`);
            }
        });

        // Sync Tree View with active editor
        const changeSelection = window.onDidChangeActiveTextEditor(editor => {
            if (editor && treeDataProvider && treeView.visible) {
                const uri = editor.document.uri;
                treeDataProvider.reveal(treeView, uri);
            }
        });

        // Also sync when the tree view becomes visible
        const changeVisibility = treeView.onDidChangeVisibility(e => {
            if (e.visible && window.activeTextEditor && treeDataProvider) {
                treeDataProvider.reveal(treeView, window.activeTextEditor.document.uri);
            }
        });

        const splitNoteCommand = commands.registerCommand('dendrite.splitNote', async (...args: any[]) => {
            try {
                let uri: any;
                let range: any;
                let newNoteName: string | undefined;

                if (args.length >= 2) {
                    // Triggered by Code Action
                    uri = args[0];
                    range = args[1];
                } else {
                    // Triggered by Command Palette
                    const editor = window.activeTextEditor;
                    if (!editor) {
                        return;
                    }
                    if (editor.selection.isEmpty) {
                        window.showWarningMessage('Please select text to extract.');
                        return;
                    }
                    uri = editor.document.uri;
                    range = editor.selection;
                }

                // Ask for new note name
                newNoteName = await window.showInputBox({
                    prompt: 'Enter the name of the new note',
                    placeHolder: 'e.g. new_note'
                });

                if (!newNoteName) {
                    return;
                }

                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/splitNote',
                    arguments: [uri, range, newNoteName]
                });
            } catch (error) {
                window.showErrorMessage(`Split Note failed: ${error}`);
            }
        });

        const reorganizeHierarchyCommand = commands.registerCommand('dendrite.reorganizeHierarchy', async () => {
            try {
                const oldKey = await window.showInputBox({
                    prompt: 'Enter the old hierarchy prefix to rename',
                    placeHolder: 'e.g. projects.inactive'
                });
                if (!oldKey) { return; }

                const newKey = await window.showInputBox({
                    prompt: 'Enter the new hierarchy prefix',
                    placeHolder: 'e.g. archive.projects'
                });
                if (!newKey) { return; }

                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/reorganizeHierarchy',
                    arguments: [oldKey, newKey]
                });
            } catch (error) {
                window.showErrorMessage(`Reorganize Hierarchy failed: ${error}`);
            }
        });

        const auditCommand = commands.registerCommand('dendrite.workspaceAudit', async () => {
            try {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/workspaceAudit',
                    arguments: []
                });
                // Server sends a window/showMessage notification on completion
                // We could also inspect the return value if specialized UI was needed
            } catch (error) {
                window.showErrorMessage(`Audit failed: ${error}`);
            }
        });

        context.subscriptions.push(treeView, refreshCommand, renameCommand, undoCommand, splitNoteCommand, reorganizeHierarchyCommand, auditCommand, changeSelection, changeVisibility);
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

