import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function registerReorganizeHierarchyCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.reorganizeHierarchy', async () => {
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
}
