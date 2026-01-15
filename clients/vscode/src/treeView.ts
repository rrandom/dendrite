import { window, commands, ExtensionContext } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { DendriteTreeDataProvider } from './treeDataProvider';

export function registerTreeView(context: ExtensionContext, client: LanguageClient) {
    const treeDataProvider = new DendriteTreeDataProvider(client);
    const treeView = window.createTreeView('dendriteHierarchy', {
        treeDataProvider: treeDataProvider,
        showCollapseAll: true
    });

    const refreshCommand = commands.registerCommand('dendrite.refreshHierarchy', () => {
        treeDataProvider.refresh();
    });

    // Sync Tree View with active editor
    const changeSelection = window.onDidChangeActiveTextEditor(editor => {
        if (editor && treeView.visible) {
            const uri = editor.document.uri;
            treeDataProvider.reveal(treeView, uri);
        }
    });

    // Also sync when the tree view becomes visible
    const changeVisibility = treeView.onDidChangeVisibility(e => {
        if (e.visible && window.activeTextEditor) {
            treeDataProvider.reveal(treeView, window.activeTextEditor.document.uri);
        }
    });

    context.subscriptions.push(treeView, refreshCommand, changeSelection, changeVisibility);

    return treeDataProvider;
}
