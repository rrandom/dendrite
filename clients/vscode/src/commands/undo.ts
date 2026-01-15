import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { applyRefactor } from '../utils';

export function registerUndoCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.undoRefactor', async () => {
        try {
            await applyRefactor(async () => {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/undoRefactor',
                    arguments: []
                });
            }, 'Undo successful');
        } catch (error) {
            window.showErrorMessage(`Undo failed: ${error}`);
        }
    });
}
