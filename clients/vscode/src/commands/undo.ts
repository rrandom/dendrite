import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function registerUndoCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.undoRefactor', async () => {
        try {
            await client.sendRequest('workspace/executeCommand', {
                command: 'dendrite/undoRefactor',
                arguments: []
            });
        } catch (error) {
            window.showErrorMessage(`Undo failed: ${error}`);
        }
    });
}
