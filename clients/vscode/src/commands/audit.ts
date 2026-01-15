import { window, commands } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function registerAuditCommand(client: LanguageClient) {
    return commands.registerCommand('dendrite.workspaceAudit', async () => {
        try {
            await client.sendRequest('workspace/executeCommand', {
                command: 'dendrite/workspaceAudit',
                arguments: []
            });
            // Server sends a window/showMessage notification on completion
            // We could also inspect the return value if specialized UI was needed
        } catch (error) {
            window.showErrorMessage(`Audit failed: ${error}`);
        }
    });
}
