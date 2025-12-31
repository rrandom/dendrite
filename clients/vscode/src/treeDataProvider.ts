import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

// Types matching the Rust backend
interface NoteRef {
    id: string;
    key: string | null;
    path: string | null;
    title: string | null;
}

interface TreeView {
    note: NoteRef;
    children: TreeView[];
}

interface GetHierarchyResult {
    roots: TreeView[];
}

// Tree item for VS Code TreeView
export class DendriteTreeItem extends vscode.TreeItem {
    constructor(
        public readonly noteRef: NoteRef,
        public readonly children: DendriteTreeItem[],
        public readonly collapsibleState: vscode.TreeItemCollapsibleState
    ) {
        // Use title if available, otherwise use key, otherwise use id
        const label = noteRef.title || noteRef.key || noteRef.id;
        super(label, collapsibleState);

        // Set tooltip
        this.tooltip = noteRef.key || noteRef.id;
        this.description = noteRef.key || undefined;

        // Set icon based on whether it has a path (real file) or not (ghost node)
        if (noteRef.path) {
            this.iconPath = vscode.ThemeIcon.File;
        } else {
            this.iconPath = new vscode.ThemeIcon('circle-outline');
        }

        // Set command to open file when clicked
        if (noteRef.path) {
            this.command = {
                command: 'vscode.open',
                title: 'Open Note',
                arguments: [vscode.Uri.parse(noteRef.path)]
            };
        }
    }
}

// TreeDataProvider implementation
export class DendriteTreeDataProvider implements vscode.TreeDataProvider<DendriteTreeItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<DendriteTreeItem | undefined | null | void> = new vscode.EventEmitter<DendriteTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<DendriteTreeItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private cachedTree: TreeView[] | null = null;

    constructor(private client: LanguageClient) {}

    refresh(): void {
        this.cachedTree = null;
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: DendriteTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: DendriteTreeItem): Promise<DendriteTreeItem[]> {
        try {
            // Get or fetch tree structure
            let treeRoots: TreeView[];
            if (!this.cachedTree) {
                // Call LSP command to get hierarchy
                const result = await this.client.sendRequest<GetHierarchyResult>(
                    'workspace/executeCommand',
                    {
                        command: 'dendrite/getHierarchy',
                        arguments: []
                    }
                );

                if (!result || !result.roots) {
                    return [];
                }

                this.cachedTree = result.roots;
            }
            treeRoots = this.cachedTree;

            // If element is provided, get its children; otherwise return root nodes
            if (element) {
                // Find the corresponding tree view node
                const treeViewNode = this.findNodeInTree(treeRoots, element.noteRef.id);
                if (treeViewNode) {
                    return treeViewNode.children.map(child => this.convertToTreeItem(child));
                }
                return [];
            } else {
                // Return root nodes
                return treeRoots.map(root => this.convertToTreeItem(root));
            }
        } catch (error) {
            console.error('Error fetching hierarchy:', error);
            vscode.window.showErrorMessage(`Failed to fetch hierarchy: ${error}`);
            return [];
        }
    }

    private findNodeInTree(roots: TreeView[], id: string): TreeView | null {
        for (const root of roots) {
            if (root.note.id === id) {
                return root;
            }
            const found = this.findNodeInTree(root.children, id);
            if (found) {
                return found;
            }
        }
        return null;
    }

    private convertToTreeItem(treeView: TreeView): DendriteTreeItem {
        const collapsibleState = treeView.children.length > 0
            ? vscode.TreeItemCollapsibleState.Collapsed
            : vscode.TreeItemCollapsibleState.None;

        const children = treeView.children.map(child => this.convertToTreeItem(child));

        return new DendriteTreeItem(
            treeView.note,
            children,
            collapsibleState
        );
    }
}

