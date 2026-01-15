import { window, commands, workspace } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function registerRenameNoteCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.renameNote', async () => {
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
}
