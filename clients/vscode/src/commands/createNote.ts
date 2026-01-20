import { window, commands, workspace, Uri } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { runWorkspaceMutation } from '../utils';

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
            await runWorkspaceMutation(async () => {
                const uriString = await client.sendRequest<string | null>('workspace/executeCommand', {
                    command: 'dendrite/createNote',
                    arguments: [key]
                });
                
                if (uriString) {
                    const uri = Uri.parse(uriString);
                    const doc = await workspace.openTextDocument(uri);
                    await window.showTextDocument(doc);
                }
            }, `Created note ${key}`);
        } catch (e) {
            window.showErrorMessage(`Failed to create note: ${e}`);
        }
    });
}
