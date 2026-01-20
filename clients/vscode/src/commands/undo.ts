import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { runWorkspaceMutation } from '../utils';

export function registerUndoCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.undoMutation', async () => {
        try {
            await runWorkspaceMutation(async () => {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/undoMutation',
                    arguments: []
                });
            }, 'Undo successful');
        } catch (error) {
            window.showErrorMessage(`Undo failed: ${error}`);
        }
    });
}
