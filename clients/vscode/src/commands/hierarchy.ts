import { window, commands, workspace } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { applyRefactor } from '../utils';
import { generatePreviewHtml } from '../preview';

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

            // 1. Dry Run / Preview
            const previewEdits = await client.sendRequest<[string, string][]>('workspace/executeCommand', {
                command: 'dendrite/resolveHierarchyEdits',
                arguments: [oldKey, newKey]
            });

            if (!previewEdits || previewEdits.length === 0) {
                window.showInformationMessage('No matching notes found for this hierarchy.');
                return;
            }

            // 2. Show Preview Table via Webview
            const panel = window.createWebviewPanel(
                'dendrite.hierarchyPreview',
                'Hierarchy Refactor Preview',
                window.activeTextEditor?.viewColumn || 1,
                {}
            );

            panel.webview.html = generatePreviewHtml({
                title: 'Hierarchy Refactor Preview',
                metadata: [
                    { label: 'From', value: oldKey },
                    { label: 'To', value: newKey }
                ],
                columns: [
                    { header: 'Old Note Key', key: 'old' },
                    { header: 'New Note Key', key: 'new' }
                ],
                rows: previewEdits.map(([oldK, newK]) => ({ old: oldK, new: newK }))
            });

            // 3. Confirm via QuickPick
            const quickPick = window.createQuickPick();
            quickPick.ignoreFocusOut = true;
            quickPick.items = [
                { label: 'Yes', description: 'Apply Changes' },
                { label: 'Cancel', description: 'Abort' }
            ];
            quickPick.placeholder = `Confirm renaming ${previewEdits.length} notes?`;
            
            // Wait for user selection
            const selection = await new Promise<string | undefined>((resolve) => {
                quickPick.onDidAccept(() => {
                    resolve(quickPick.selectedItems[0]?.label);
                    quickPick.hide();
                });
                quickPick.onDidHide(() => resolve(undefined)); // Cancel if dismissed
                quickPick.show();
            });

            // Close webview
            panel.dispose();
            quickPick.dispose();

            if (selection !== 'Yes') {
                return;
            }

            // 4. Execute
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
