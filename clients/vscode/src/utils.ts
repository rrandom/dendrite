import { commands, workspace, window } from 'vscode';

/**
 * Helper to execute a workspace mutation operation (e.g., refactoring, creation).
 * It ensures files are saved and the hierarchy view is refreshed after the operation.
 * 
 * @param operation The async operation to execute (e.g., sending an LSP request).
 * @param successMessage Optional message to show on success.
 */
export async function runWorkspaceMutation(operation: () => Promise<void>, successMessage?: string) {
    try {
        await operation();
        
        // Save all changes made by the workspace edit
        // We accept a small delay to ensure VSCode has marked files as dirty
        for (let i = 0; i < 5; i++) {
            const dirtyDocs = workspace.textDocuments.filter(doc => doc.isDirty);
            if (dirtyDocs.length === 0 && i > 0) break; // If no dirty docs (and we waited at least once), done.

            if (dirtyDocs.length > 0 || i === 0) {
                 console.log(`[Dendrite] Saving workspace mutation (Attempt ${i + 1}). Dirty docs: ${dirtyDocs.length}`);
                 await workspace.saveAll();
            }
            await new Promise(resolve => setTimeout(resolve, 100));
        }
        console.log('[Dendrite] Workspace mutation completed and saved.');
        
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
