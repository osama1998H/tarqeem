/**
 * RTL (Right-to-Left) Support Utilities for Tarqeem
 * دعم الاتجاه من اليمين لليسار لترقيم
 *
 * This module provides utilities for better Arabic text handling in VS Code.
 */

import * as vscode from 'vscode';

// Unicode ranges for Arabic characters
const ARABIC_RANGES = [
    [0x0600, 0x06FF],  // Arabic
    [0x0750, 0x077F],  // Arabic Supplement
    [0x08A0, 0x08FF],  // Arabic Extended-A
    [0xFB50, 0xFDFF],  // Arabic Presentation Forms-A
    [0xFE70, 0xFEFF],  // Arabic Presentation Forms-B
];

// Arabic-Indic digits
const ARABIC_DIGITS: { [key: string]: string } = {
    '٠': '0', '١': '1', '٢': '2', '٣': '3', '٤': '4',
    '٥': '5', '٦': '6', '٧': '7', '٨': '8', '٩': '9',
};

/**
 * Configure RTL support for the extension
 */
export function configureRTLSupport(context: vscode.ExtensionContext): void {
    // Register completion item provider with RTL-aware formatting
    const completionProvider = createRTLAwareCompletionProvider();
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            'tarqeem',
            completionProvider,
            '.', ':', '(' // Trigger characters
        )
    );

    // Apply workspace configuration for Tarqeem files
    applyRTLConfiguration();

    // Register document change listener for RTL adjustments
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument((document) => {
            if (document.languageId === 'tarqeem') {
                applyRTLSettings(document);
            }
        })
    );

    // Apply to already open documents
    vscode.workspace.textDocuments.forEach((document) => {
        if (document.languageId === 'tarqeem') {
            applyRTLSettings(document);
        }
    });
}

/**
 * Apply RTL-specific workspace configuration
 */
function applyRTLConfiguration(): void {
    const config = vscode.workspace.getConfiguration('tarqeem');

    // Check if unicode highlighting should be disabled
    if (!config.get<boolean>('editor.unicodeHighlight', false)) {
        // Update editor settings for tarqeem language
        const editorConfig = vscode.workspace.getConfiguration('editor', { languageId: 'tarqeem' });

        // These settings help with Arabic display
        try {
            // Disable ambiguous character highlighting
            editorConfig.update('unicodeHighlight.ambiguousCharacters', false, vscode.ConfigurationTarget.Workspace);

            // Disable invisible character highlighting (some Arabic diacritics)
            editorConfig.update('unicodeHighlight.invisibleCharacters', false, vscode.ConfigurationTarget.Workspace);

            // Allow non-basic ASCII characters
            editorConfig.update('unicodeHighlight.nonBasicASCII', false, vscode.ConfigurationTarget.Workspace);
        } catch {
            // Configuration updates may fail if workspace isn't writable
        }
    }
}

/**
 * Apply RTL settings to a specific document
 */
function applyRTLSettings(document: vscode.TextDocument): void {
    // Check if document contains Arabic text
    const hasArabic = containsArabic(document.getText());

    if (hasArabic) {
        // Apply visual adjustments for Arabic content
        const editor = vscode.window.visibleTextEditors.find(
            (e) => e.document === document
        );

        if (editor) {
            // Additional editor-specific settings can be applied here
            // Note: VS Code doesn't have full RTL support, so we do our best
        }
    }
}

/**
 * Create a completion provider that handles RTL text properly
 */
function createRTLAwareCompletionProvider(): vscode.CompletionItemProvider {
    return {
        provideCompletionItems(
            document: vscode.TextDocument,
            position: vscode.Position,
            _token: vscode.CancellationToken,
            _context: vscode.CompletionContext
        ): vscode.CompletionItem[] | undefined {
            // This provider adds RTL-aware completions
            // The main completions come from the LSP server
            // This is just for additional local completions if needed

            const items: vscode.CompletionItem[] = [];

            // Check context
            const line = document.lineAt(position.line).text;

            // If we're in a context where we might want Arabic keywords
            if (isArabicContext(line, position.character)) {
                // The LSP server handles most completions
                // This is for any client-side additions
            }

            return items.length > 0 ? items : undefined;
        },
    };
}

/**
 * Check if the text contains Arabic characters
 */
export function containsArabic(text: string): boolean {
    for (const char of text) {
        if (isArabicChar(char)) {
            return true;
        }
    }
    return false;
}

/**
 * Check if a character is Arabic
 */
export function isArabicChar(char: string): boolean {
    const code = char.charCodeAt(0);
    for (const [start, end] of ARABIC_RANGES) {
        if (code >= start && code <= end) {
            return true;
        }
    }
    return false;
}

/**
 * Check if we're in an Arabic context (for completion)
 */
function isArabicContext(line: string, position: number): boolean {
    // Look at characters before the cursor
    for (let i = position - 1; i >= 0; i--) {
        const char = line[i];
        if (char === ' ' || char === '\t') {
            continue;
        }
        return isArabicChar(char);
    }
    return false;
}

/**
 * Convert Arabic-Indic digits to Western digits
 */
export function arabicToWesternDigits(text: string): string {
    let result = '';
    for (const char of text) {
        result += ARABIC_DIGITS[char] || char;
    }
    return result;
}

/**
 * Normalize Arabic text (NFC normalization)
 */
export function normalizeArabicText(text: string): string {
    return text.normalize('NFC');
}

/**
 * Get the directionality of a string
 */
export function getTextDirection(text: string): 'ltr' | 'rtl' | 'mixed' {
    let hasLTR = false;
    let hasRTL = false;

    for (const char of text) {
        if (isArabicChar(char)) {
            hasRTL = true;
        } else if (/[a-zA-Z]/.test(char)) {
            hasLTR = true;
        }

        if (hasLTR && hasRTL) {
            return 'mixed';
        }
    }

    if (hasRTL && !hasLTR) {
        return 'rtl';
    }
    if (hasLTR && !hasRTL) {
        return 'ltr';
    }
    return 'mixed';
}

/**
 * Insert bidirectional control characters for proper display
 * LRE = Left-to-Right Embedding (U+202A)
 * RLE = Right-to-Left Embedding (U+202B)
 * PDF = Pop Directional Formatting (U+202C)
 */
export function wrapWithBidiControls(text: string, direction: 'ltr' | 'rtl'): string {
    const LRE = '\u202A';
    const RLE = '\u202B';
    const PDF = '\u202C';

    if (direction === 'ltr') {
        return `${LRE}${text}${PDF}`;
    } else {
        return `${RLE}${text}${PDF}`;
    }
}

/**
 * Strip bidirectional control characters
 */
export function stripBidiControls(text: string): string {
    return text.replace(/[\u200E\u200F\u202A-\u202E\u2066-\u2069]/g, '');
}

/**
 * Check if a string is valid as a Tarqeem identifier
 */
export function isValidIdentifier(text: string): boolean {
    if (text.length === 0) {
        return false;
    }

    // First character must be letter (Arabic or Latin) or underscore
    const firstChar = text[0];
    const isValidFirst =
        isArabicChar(firstChar) ||
        /[a-zA-Z_]/.test(firstChar);

    if (!isValidFirst) {
        return false;
    }

    // Remaining characters can also include digits
    for (let i = 1; i < text.length; i++) {
        const char = text[i];
        const isValid =
            isArabicChar(char) ||
            /[a-zA-Z0-9_]/.test(char);
        if (!isValid) {
            return false;
        }
    }

    return true;
}

/**
 * Create decoration type for RTL hints (if needed)
 */
export function createRTLDecorations(_context: vscode.ExtensionContext): vscode.TextEditorDecorationType {
    return vscode.window.createTextEditorDecorationType({
        // Visual hints for RTL content can be added here
        // For example, subtle background highlighting for Arabic text
        // Currently minimal to avoid visual noise
    });
}
