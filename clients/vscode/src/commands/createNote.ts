import { window, commands, workspace, Uri } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as path from 'path';
import { applyRefactor } from '../utils';


export function registerCreateNoteCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.createNote', async () => {
        const key = await window.showInputBox({
            title: 'Create Note',
            prompt: 'Enter Note Key (e.g. foo.bar)',
            placeHolder: 'foo.bar'
        });

        if (!key) {
            return;
        }

        try {
            // Execute command on server. 
            // The server will trigger a workspace/applyEdit to create the file.
            await applyRefactor(async () => {
                const sent = await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/createNote',
                    arguments: [{ note_key: key }]
                });

                if (sent) {
                    // Determine the path to open.
                    // We assume default logic: root + key.replace('.', '/') + '.md'
                    // This logic must match the server's logic to open the correct file.
                    // Ideally server returns the URI, but we designed it to return boolean.

                    const rootPath = workspace.workspaceFolders?.[0].uri.fsPath;
                    if (rootPath) {
                        const relativePath = key.replace(/\./g, '/') + '.md';
                        const filePath = path.join(rootPath, relativePath);
                        // Wait slightly for FS? applyRefactor does saveAll.
                        const doc = await workspace.openTextDocument(Uri.file(filePath));
                        await window.showTextDocument(doc);
                    }
                }
            }, `Created note ${key}`);
        } catch (e) {
            window.showErrorMessage(`Failed to create note: ${e}`);
        }
    });
}

