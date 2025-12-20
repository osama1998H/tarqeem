/**
 * Tarqeem VS Code Commands
 * أوامر ترقيم لـ VS Code
 *
 * This module registers all VS Code commands for Tarqeem.
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { TarqeemLanguageClient } from './client';

// Terminal for running Tarqeem commands
let terminal: vscode.Terminal | undefined;

/**
 * Register all Tarqeem commands
 */
export function registerCommands(
    context: vscode.ExtensionContext,
    client: TarqeemLanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    const log = (message: string) => {
        outputChannel.appendLine(`[Commands] ${message}`);
    };

    // Get tarqeem executable path from config
    const getTarqeemPath = (): string => {
        const config = vscode.workspace.getConfiguration('tarqeem');
        return config.get<string>('server.path', 'tarqeem');
    };

    // Get or create terminal
    const getTerminal = (): vscode.Terminal => {
        if (terminal === undefined || terminal.exitStatus !== undefined) {
            terminal = vscode.window.createTerminal({
                name: 'ترقيم - Tarqeem',
                iconPath: new vscode.ThemeIcon('terminal'),
            });
        }
        return terminal;
    };

    // Register run command (Arabic)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.run', async () => {
            await runCurrentFile(getTarqeemPath, getTerminal, log);
        })
    );

    // Register run command (English alias)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.run.en', async () => {
            await runCurrentFile(getTarqeemPath, getTerminal, log);
        })
    );

    // Register compile command (Arabic)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.compile', async () => {
            await compileCurrentFile(getTarqeemPath, getTerminal, log);
        })
    );

    // Register compile command (English alias)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.compile.en', async () => {
            await compileCurrentFile(getTarqeemPath, getTerminal, log);
        })
    );

    // Register format command
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.format', async () => {
            await formatCurrentFile(client, log);
        })
    );

    // Register format command (English alias)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.format.en', async () => {
            await formatCurrentFile(client, log);
        })
    );

    // Register check command
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.check', async () => {
            await checkCurrentFile(getTarqeemPath, getTerminal, log);
        })
    );

    // Register new project command
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.newProject', async () => {
            await createNewProject(getTarqeemPath, log);
        })
    );

    // Register new project command (English alias)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.newProject.en', async () => {
            await createNewProject(getTarqeemPath, log);
        })
    );

    // Register REPL command
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.openRepl', async () => {
            await openRepl(getTarqeemPath, getTerminal, log);
        })
    );

    // Register REPL command (English alias)
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.openRepl.en', async () => {
            await openRepl(getTarqeemPath, getTerminal, log);
        })
    );

    // Register restart server command
    context.subscriptions.push(
        vscode.commands.registerCommand('tarqeem.restartServer', async () => {
            log('Restarting language server...');
            await client.restart();
            vscode.window.showInformationMessage(
                'Tarqeem language server restarted - تم إعادة تشغيل خادم ترقيم'
            );
        })
    );

    log('All commands registered');
}

/**
 * Run the current Tarqeem file
 */
async function runCurrentFile(
    getTarqeemPath: () => string,
    getTerminal: () => vscode.Terminal,
    log: (msg: string) => void
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage(
            'No file is open - لا يوجد ملف مفتوح'
        );
        return;
    }

    const document = editor.document;
    if (document.languageId !== 'tarqeem') {
        vscode.window.showWarningMessage(
            'Not a Tarqeem file - ليس ملف ترقيم'
        );
        return;
    }

    // Save the file first
    await document.save();

    const filePath = document.uri.fsPath;
    const tarqeemPath = getTarqeemPath();
    const terminal = getTerminal();

    log(`Running: ${filePath}`);

    // Show terminal and run
    terminal.show();
    terminal.sendText(`${tarqeemPath} run "${filePath}"`);

    // Show output panel if configured
    const config = vscode.workspace.getConfiguration('tarqeem');
    if (config.get<boolean>('output.showOnRun', true)) {
        terminal.show();
    }
}

/**
 * Compile the current Tarqeem file
 */
async function compileCurrentFile(
    getTarqeemPath: () => string,
    getTerminal: () => vscode.Terminal,
    log: (msg: string) => void
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage(
            'No file is open - لا يوجد ملف مفتوح'
        );
        return;
    }

    const document = editor.document;
    if (document.languageId !== 'tarqeem') {
        vscode.window.showWarningMessage(
            'Not a Tarqeem file - ليس ملف ترقيم'
        );
        return;
    }

    // Save the file first
    await document.save();

    const filePath = document.uri.fsPath;
    const tarqeemPath = getTarqeemPath();
    const terminal = getTerminal();

    // Determine output path
    const dir = path.dirname(filePath);
    const baseName = path.basename(filePath, path.extname(filePath));
    const outputPath = path.join(dir, baseName);

    log(`Compiling: ${filePath} -> ${outputPath}`);

    // Show terminal and compile
    terminal.show();
    terminal.sendText(`${tarqeemPath} compile "${filePath}" -o "${outputPath}"`);
}

/**
 * Format the current Tarqeem file using LSP
 */
async function formatCurrentFile(
    client: TarqeemLanguageClient,
    log: (msg: string) => void
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage(
            'No file is open - لا يوجد ملف مفتوح'
        );
        return;
    }

    const document = editor.document;
    if (document.languageId !== 'tarqeem') {
        vscode.window.showWarningMessage(
            'Not a Tarqeem file - ليس ملف ترقيم'
        );
        return;
    }

    log(`Formatting: ${document.uri.fsPath}`);

    try {
        // Use VS Code's built-in format command which will use our LSP
        const success = await vscode.commands.executeCommand(
            'editor.action.formatDocument'
        );

        if (success !== false) {
            log('File formatted successfully');
        }
    } catch (error) {
        log(`Format error: ${error}`);
        vscode.window.showErrorMessage(
            `Failed to format file - فشل في تنسيق الملف: ${error}`
        );
    }
}

/**
 * Check the current Tarqeem file for errors
 */
async function checkCurrentFile(
    getTarqeemPath: () => string,
    getTerminal: () => vscode.Terminal,
    log: (msg: string) => void
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage(
            'No file is open - لا يوجد ملف مفتوح'
        );
        return;
    }

    const document = editor.document;
    if (document.languageId !== 'tarqeem') {
        vscode.window.showWarningMessage(
            'Not a Tarqeem file - ليس ملف ترقيم'
        );
        return;
    }

    // Save the file first
    await document.save();

    const filePath = document.uri.fsPath;
    const tarqeemPath = getTarqeemPath();
    const terminal = getTerminal();

    log(`Checking: ${filePath}`);

    terminal.show();
    terminal.sendText(`${tarqeemPath} check "${filePath}"`);
}

/**
 * Create a new Tarqeem project
 */
async function createNewProject(
    getTarqeemPath: () => string,
    log: (msg: string) => void
): Promise<void> {
    // Ask for project name
    const projectName = await vscode.window.showInputBox({
        prompt: 'Enter project name - أدخل اسم المشروع',
        placeHolder: 'my-project / مشروعي',
        validateInput: (value) => {
            if (!value || value.trim().length === 0) {
                return 'Project name is required - اسم المشروع مطلوب';
            }
            // Allow Arabic and English names
            if (!/^[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FFa-zA-Z][\u0600-\u06FF\u0750-\u077F\u08A0-\u08FFa-zA-Z0-9_-]*$/.test(value)) {
                return 'Invalid project name - اسم المشروع غير صالح';
            }
            return null;
        },
    });

    if (!projectName) {
        return;
    }

    // Ask for project location
    const folderUri = await vscode.window.showOpenDialog({
        canSelectFiles: false,
        canSelectFolders: true,
        canSelectMany: false,
        openLabel: 'Select Location - اختر الموقع',
    });

    if (!folderUri || folderUri.length === 0) {
        return;
    }

    const projectPath = path.join(folderUri[0].fsPath, projectName);
    const tarqeemPath = getTarqeemPath();

    log(`Creating project: ${projectPath}`);

    try {
        // Create project using trqpm or tarqeem init
        const terminal = vscode.window.createTerminal({
            name: `Create ${projectName}`,
            cwd: folderUri[0].fsPath,
        });

        terminal.show();
        // Try trqpm first, fall back to mkdir and manual creation
        terminal.sendText(`mkdir -p "${projectName}" && cd "${projectName}" && ${tarqeemPath} init . 2>/dev/null || (echo "Creating project manually..." && mkdir -p مصدر && echo 'اطبع("مرحباً بالعالم!")' > مصدر/رئيسي.trq)`);

        // Ask to open the project
        const action = await vscode.window.showInformationMessage(
            `Project created: ${projectName} - تم إنشاء المشروع`,
            'Open Project - فتح المشروع',
            'Close - إغلاق'
        );

        if (action === 'Open Project - فتح المشروع') {
            await vscode.commands.executeCommand(
                'vscode.openFolder',
                vscode.Uri.file(projectPath)
            );
        }
    } catch (error) {
        log(`Error creating project: ${error}`);
        vscode.window.showErrorMessage(
            `Failed to create project - فشل في إنشاء المشروع: ${error}`
        );
    }
}

/**
 * Open Tarqeem REPL
 */
async function openRepl(
    getTarqeemPath: () => string,
    getTerminal: () => vscode.Terminal,
    log: (msg: string) => void
): Promise<void> {
    const tarqeemPath = getTarqeemPath();

    log('Opening REPL...');

    // Create a new terminal specifically for REPL
    const replTerminal = vscode.window.createTerminal({
        name: 'ترقيم REPL',
        iconPath: new vscode.ThemeIcon('terminal-cmd'),
    });

    replTerminal.show();
    replTerminal.sendText(`${tarqeemPath} repl`);

    vscode.window.showInformationMessage(
        'Tarqeem REPL started - بدأت وحدة REPL'
    );
}

/**
 * Dispose terminal when extension deactivates
 */
export function disposeTerminal(): void {
    if (terminal) {
        terminal.dispose();
        terminal = undefined;
    }
}
