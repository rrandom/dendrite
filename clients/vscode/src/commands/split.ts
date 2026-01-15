import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { applyRefactor } from '../utils';

export function registerSplitNoteCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.splitNote', async (...args: any[]) => {
        try {
            let uri: any;
            let range: any;
            let newNoteName: string | undefined;

            if (args.length >= 2) {
                // Triggered by Code Action
                uri = args[0];
                range = args[1];
            } else {
                // Triggered by Command Palette
                const editor = window.activeTextEditor;
                if (!editor) {
                    return;
                }
                if (editor.selection.isEmpty) {
                    window.showWarningMessage('Please select text to extract.');
                    return;
                }
                uri = editor.document.uri;
                range = editor.selection;
            }

            // Ask for new note name
            newNoteName = await window.showInputBox({
                prompt: 'Enter the name of the new note',
                placeHolder: 'e.g. new_note'
            });

            if (!newNoteName) {
                return;
            }

            await applyRefactor(async () => {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'dendrite/splitNote',
                    arguments: [uri, range, newNoteName]
                });
            }, `Extracted to ${newNoteName}`);
        } catch (error) {
            window.showErrorMessage(`Split Note failed: ${error}`);
        }
    });
}
