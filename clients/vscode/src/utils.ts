import { commands, workspace, window } from 'vscode';

/**
 * Helper to execute a refactoring operation that involves server-side workspace edits.
 * It ensures files are saved and the hierarchy view is refreshed after the operation.
 * 
 * @param operation The async operation to execute (e.g., sending an LSP request).
 * @param successMessage Optional message to show on success.
 */
export async function applyRefactor(operation: () => Promise<void>, successMessage?: string) {
    try {
        await operation();
        
        // Save all changes made by the workspace edit
        await workspace.saveAll();
        
        // Refresh the hierarchy view
        await commands.executeCommand('dendrite.refreshHierarchy');

        if (successMessage) {
            window.setStatusBarMessage(successMessage, 3000);
        }
    } catch (error) {
        window.showErrorMessage(`Operation failed: ${error}`);
        throw error; // Re-throw if caller needs to handle it, though usually we handle UI here
    }
}
