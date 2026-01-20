import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { runWorkspaceMutation } from '../utils';
import { generatePreviewHtml } from '../preview';

interface Backlink {
    key: string;
    title: string | null;
    uri: string | null;
}

interface GetBacklinksResult {
    backlinks: Backlink[];
}

export function registerDeleteNoteCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.deleteNote', async () => {
        try {
            const editor = window.activeTextEditor;
            if (!editor) {
                window.showErrorMessage('No active editor.');
                return;
            }

            // 1. Resolve Note Key from current document
            const uri = editor.document.uri.toString();
            // TODO: We need a way to get the note key from the URI directly or from LSP
            // For now, let's ask the user to confirm the key or use a new LSP request "getNoteKey" if available
            // Actually, existing getNoteKey command is available: dendrite/getNoteKey
            
            const keyResult = await client.sendRequest<{ key: string }>('workspace/executeCommand', {
                command: 'dendrite/getNoteKey',
                arguments: [{ uri }]
            });

            if (!keyResult || !keyResult.key) {
               window.showErrorMessage('Could not resolve note key for current file.'); 
               return;
            }
            const noteKey = keyResult.key;

            // 2. Check Backlinks (Safety Check)
            const backlinksResult = await client.sendRequest<GetBacklinksResult>('workspace/executeCommand', {
                command: 'dendrite/getBacklinks',
                arguments: [{ note_key: noteKey }]
            });

            const backlinks = backlinksResult?.backlinks || [];

            if (backlinks.length > 0) {
                // 3a. Show Warning Webview
                const panel = window.createWebviewPanel(
                    'dendrite.deleteWarning',
                    'Delete Note Warning',
                    window.activeTextEditor?.viewColumn || 1,
                    {}
                );

                panel.webview.html = generatePreviewHtml({
                    title: `⚠️ Delete '${noteKey}'?`,
                    metadata: [
                        { label: 'Note', value: noteKey },
                        { label: 'Warning', value: `This note is linked by ${backlinks.length} other note(s). Deleting it will create broken links.` }
                    ],
                    columns: [
                        { header: 'Linking Note Key', key: 'key' },
                        { header: 'Title', key: 'title' }
                    ],
                    rows: backlinks.map(b => ({ key: b.key, title: b.title || '(No Title)' }))
                });

                // Confirm via QuickPick (Modal)
                const quickPick = window.createQuickPick();
                quickPick.ignoreFocusOut = true;
                quickPick.items = [
                    { label: 'Yes, Delete', description: 'I understand this breaks links', detail: 'This action can be undone.' },
                    { label: 'Cancel', description: 'Abort deletion' }
                ];
                quickPick.placeholder = `Confirm deletion of '${noteKey}'?`;
                
                const selection = await new Promise<string | undefined>((resolve) => {
                    quickPick.onDidAccept(() => {
                        resolve(quickPick.selectedItems[0]?.label);
                        quickPick.hide();
                    });
                    quickPick.onDidHide(() => resolve(undefined)); 
                    quickPick.show();
                });

                panel.dispose();
                quickPick.dispose();

                if (selection !== 'Yes, Delete') {
                    return;
                }
            } else {
                // 3b. Simple Confirmation
                const confirm = await window.showWarningMessage(
                    `Are you sure you want to delete '${noteKey}'?`,
                    { modal: true },
                    'Delete'
                );
                if (confirm !== 'Delete') {
                    return;
                }
            }

            // 4. Execute Deletion
            await runWorkspaceMutation(async () => {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/deleteNote',
                    arguments: [{ note_key: noteKey }]
                });
            }, `Deleted note '${noteKey}'`);

        } catch (error) {
            window.showErrorMessage(`Delete Note failed: ${error}`);
        }
    });
}
