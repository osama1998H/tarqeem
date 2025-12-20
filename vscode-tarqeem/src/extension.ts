/**
 * Tarqeem VS Code Extension - Main Entry Point
 * إضافة ترقيم لـ VS Code - نقطة الدخول الرئيسية
 *
 * This extension provides full IDE support for the Tarqeem programming language,
 * including syntax highlighting, auto-completion, diagnostics, and more.
 */

import * as vscode from 'vscode';
import { TarqeemLanguageClient } from './client';
import { registerCommands } from './commands';
import { configureRTLSupport } from './rtl';

// Global language client instance
let client: TarqeemLanguageClient | undefined;

// Output channel for extension logs
let outputChannel: vscode.OutputChannel;

/**
 * Activate the extension
 * تفعيل الإضافة
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    // Create output channel for logging
    outputChannel = vscode.window.createOutputChannel('ترقيم - Tarqeem');
    context.subscriptions.push(outputChannel);

    log('Activating Tarqeem extension...');
    log('تفعيل إضافة ترقيم...');

    try {
        // Configure RTL support for Arabic text
        configureRTLSupport(context);

        // Create and start the language client
        client = new TarqeemLanguageClient(context, outputChannel);
        await client.start();

        // Register all commands
        registerCommands(context, client, outputChannel);

        // Apply editor configuration for Tarqeem files
        applyEditorConfiguration(context);

        // Watch for configuration changes
        context.subscriptions.push(
            vscode.workspace.onDidChangeConfiguration((event) => {
                if (event.affectsConfiguration('tarqeem')) {
                    handleConfigurationChange(event);
                }
            })
        );

        // Log successful activation
        log('Tarqeem extension activated successfully!');
        log('تم تفعيل إضافة ترقيم بنجاح!');

        // Show welcome message on first activation
        showWelcomeMessage(context);

    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        log(`Failed to activate extension: ${errorMessage}`);
        log(`فشل في تفعيل الإضافة: ${errorMessage}`);
        vscode.window.showErrorMessage(
            `Tarqeem: Failed to start language server. ${errorMessage}\n` +
            `ترقيم: فشل في بدء خادم اللغة. ${errorMessage}`
        );
    }
}

/**
 * Deactivate the extension
 * إلغاء تفعيل الإضافة
 */
export async function deactivate(): Promise<void> {
    log('Deactivating Tarqeem extension...');
    log('إلغاء تفعيل إضافة ترقيم...');

    if (client) {
        await client.stop();
        client = undefined;
    }

    log('Tarqeem extension deactivated.');
    log('تم إلغاء تفعيل إضافة ترقيم.');
}

/**
 * Apply VS Code editor configuration for Tarqeem files
 */
function applyEditorConfiguration(context: vscode.ExtensionContext): void {
    const config = vscode.workspace.getConfiguration('tarqeem');

    // Disable unicode highlighting for Arabic characters if configured
    if (!config.get<boolean>('editor.unicodeHighlight', false)) {
        // These settings help with Arabic text display
        context.subscriptions.push(
            vscode.workspace.onDidOpenTextDocument((document) => {
                if (document.languageId === 'tarqeem') {
                    // Configure editor for better Arabic support
                    vscode.commands.executeCommand('editor.action.toggleRenderWhitespace', false);
                }
            })
        );
    }

    // Set up format on save if enabled
    if (config.get<boolean>('formatting.formatOnSave', false)) {
        context.subscriptions.push(
            vscode.workspace.onWillSaveTextDocument((event) => {
                if (event.document.languageId === 'tarqeem') {
                    const edit = vscode.commands.executeCommand(
                        'editor.action.formatDocument'
                    );
                    if (edit) {
                        event.waitUntil(edit.then(() => []));
                    }
                }
            })
        );
    }
}

/**
 * Handle configuration changes
 */
function handleConfigurationChange(event: vscode.ConfigurationChangeEvent): void {
    const config = vscode.workspace.getConfiguration('tarqeem');

    // Restart server if server settings changed
    if (event.affectsConfiguration('tarqeem.server')) {
        vscode.window.showInformationMessage(
            'Tarqeem server settings changed. Restart VS Code to apply.\n' +
            'تغيرت إعدادات خادم ترقيم. أعد تشغيل VS Code للتطبيق.',
            'Restart Now'
        ).then((selection) => {
            if (selection === 'Restart Now') {
                vscode.commands.executeCommand('workbench.action.reloadWindow');
            }
        });
    }

    // Update trace level
    if (event.affectsConfiguration('tarqeem.trace.server')) {
        const traceLevel = config.get<string>('trace.server', 'off');
        log(`Trace level changed to: ${traceLevel}`);
    }
}

/**
 * Show welcome message on first activation
 */
function showWelcomeMessage(context: vscode.ExtensionContext): void {
    const hasShownWelcome = context.globalState.get<boolean>('tarqeem.welcomeShown', false);

    if (!hasShownWelcome) {
        vscode.window.showInformationMessage(
            'مرحباً بك في ترقيم! Welcome to Tarqeem!',
            'عرض التوثيق / View Docs',
            'إغلاق / Close'
        ).then((selection) => {
            if (selection === 'عرض التوثيق / View Docs') {
                vscode.env.openExternal(
                    vscode.Uri.parse('https://github.com/osama1998H/tarqeem')
                );
            }
        });

        context.globalState.update('tarqeem.welcomeShown', true);
    }
}

/**
 * Log a message to the output channel
 */
function log(message: string): void {
    const timestamp = new Date().toISOString();
    outputChannel.appendLine(`[${timestamp}] ${message}`);
}

/**
 * Get the language client (for testing or external access)
 */
export function getClient(): TarqeemLanguageClient | undefined {
    return client;
}
