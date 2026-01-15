
export interface PreviewColumn {
    header: string;
    key: string;
}

export interface PreviewData {
    [key: string]: string;
}

export interface WebviewOptions {
    title: string;
    metadata: { label: string; value: string; }[];
    columns: PreviewColumn[];
    rows: PreviewData[];
}

export function generatePreviewHtml(options: WebviewOptions): string {
    const metaHtml = options.metadata.map(m => `
        <div style="margin-bottom: 8px;">
            <strong>${m.label}:</strong> <code>${m.value}</code>
        </div>
    `).join('');

    const headersHtml = options.columns.map(c => `<th>${c.header}</th>`).join('');

    const rowsHtml = options.rows.map(row => {
        const cells = options.columns.map(c => `<td>${row[c.key]}</td>`).join('');
        return `<tr>${cells}</tr>`;
    }).join('');

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>${options.title}</title>
    <style>
        body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); background-color: var(--vscode-editor-background); padding: 20px; }
        h2 { color: var(--vscode-textLink-foreground); }
        table { width: 100%; border-collapse: collapse; margin-top: 20px; }
        th, td { text-align: left; padding: 8px; border-bottom: 1px solid var(--vscode-panel-border); }
        th { color: var(--vscode-descriptionForeground); }
        tr:hover { background-color: var(--vscode-list-hoverBackground); }
        .meta { margin-bottom: 20px; font-size: 1.1em; }
        code { font-family: var(--vscode-editor-font-family); background-color: var(--vscode-textBlockQuote-background); padding: 2px 4px; border-radius: 3px; }
    </style>
</head>
<body>
    <h2>${options.title}</h2>
    <div class="meta">
        ${metaHtml}
    </div>
    <table>
        <thead>
            <tr>
                ${headersHtml}
            </tr>
        </thead>
        <tbody>
            ${rowsHtml}
        </tbody>
    </table>
</body>
</html>`;
}
