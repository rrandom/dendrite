import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { applyRefactor } from '../utils';

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

            await applyRefactor(async () => {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/reorganizeHierarchy',
                    arguments: [oldKey, newKey]
                });
            }, `Reorganized hierarchy from ${oldKey} to ${newKey}`);
        } catch (error) {
            window.showErrorMessage(`Reorganize Hierarchy failed: ${error}`);
        }
    });
}
