/**
 * Tarqeem Language Client
 * عميل لغة ترقيم
 *
 * This module implements the LSP client that connects to the Tarqeem language server.
 */

import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    Executable,
    State,
    RevealOutputChannelOn,
} from 'vscode-languageclient/node';

/**
 * Tarqeem Language Client wrapper
 */
export class TarqeemLanguageClient {
    private client: LanguageClient | undefined;
    private context: vscode.ExtensionContext;
    private outputChannel: vscode.OutputChannel;
    private statusBarItem: vscode.StatusBarItem;

    constructor(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
        this.context = context;
        this.outputChannel = outputChannel;

        // Create status bar item
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.text = '$(sync~spin) ترقيم';
        this.statusBarItem.tooltip = 'Tarqeem Language Server - جاري التشغيل';
        context.subscriptions.push(this.statusBarItem);
    }

    /**
     * Start the language client
     */
    async start(): Promise<void> {
        const config = vscode.workspace.getConfiguration('tarqeem');

        // Get server executable path
        const serverPath = this.getServerPath(config);
        const serverArgs = config.get<string[]>('server.args', ['lsp']);

        this.log(`Starting Tarqeem LSP server: ${serverPath} ${serverArgs.join(' ')}`);

        // Define server options
        const serverOptions: ServerOptions = this.createServerOptions(serverPath, serverArgs);

        // Define client options
        const clientOptions: LanguageClientOptions = this.createClientOptions(config);

        // Create the language client
        this.client = new LanguageClient(
            'tarqeem',
            'ترقيم - Tarqeem Language Server',
            serverOptions,
            clientOptions
        );

        // Register state change handler
        this.client.onDidChangeState((event) => {
            this.handleStateChange(event.newState);
        });

        // Start the client
        try {
            await this.client.start();
            this.log('Language server started successfully');
            this.showStatusBar(true);
        } catch (error) {
            this.log(`Failed to start language server: ${error}`);
            this.showStatusBar(false);
            throw error;
        }
    }

    /**
     * Stop the language client
     */
    async stop(): Promise<void> {
        if (this.client) {
            this.log('Stopping language server...');
            await this.client.stop();
            this.client = undefined;
            this.statusBarItem.hide();
            this.log('Language server stopped');
        }
    }

    /**
     * Restart the language client
     */
    async restart(): Promise<void> {
        this.log('Restarting language server...');
        await this.stop();
        await this.start();
        this.log('Language server restarted');
    }

    /**
     * Check if the client is running
     */
    isRunning(): boolean {
        return this.client !== undefined && this.client.isRunning();
    }

    /**
     * Get the underlying LanguageClient
     */
    getClient(): LanguageClient | undefined {
        return this.client;
    }

    /**
     * Get the server executable path
     */
    private getServerPath(config: vscode.WorkspaceConfiguration): string {
        const serverPath = config.get<string>('server.path', 'tarqeem');

        // The path is used as-is - it can be:
        // 1. An absolute path to the tarqeem binary
        // 2. A relative path (resolved from workspace)
        // 3. Just "tarqeem" which will be found in PATH
        // The server spawning will handle the actual resolution

        return serverPath;
    }

    /**
     * Create server options
     */
    private createServerOptions(serverPath: string, args: string[]): ServerOptions {
        const runExecutable: Executable = {
            command: serverPath,
            args: args,
            options: {
                env: {
                    ...process.env,
                    RUST_BACKTRACE: '1',
                },
            },
        };

        const debugExecutable: Executable = {
            command: serverPath,
            args: [...args, '-v'],
            options: {
                env: {
                    ...process.env,
                    RUST_BACKTRACE: 'full',
                    RUST_LOG: 'debug',
                },
            },
        };

        return {
            run: runExecutable,
            debug: debugExecutable,
        };
    }

    /**
     * Create client options
     */
    private createClientOptions(config: vscode.WorkspaceConfiguration): LanguageClientOptions {
        // Get language preference
        const languagePref = config.get<string>('language', 'auto');
        const locale = this.resolveLanguage(languagePref);

        return {
            documentSelector: [
                { scheme: 'file', language: 'tarqeem' },
                { scheme: 'untitled', language: 'tarqeem' },
            ],
            synchronize: {
                // Watch for Tarqeem configuration files
                fileEvents: [
                    vscode.workspace.createFileSystemWatcher('**/*.trq'),
                    vscode.workspace.createFileSystemWatcher('**/*.ترقيم'),
                    vscode.workspace.createFileSystemWatcher('**/*.trqh'),
                    vscode.workspace.createFileSystemWatcher('**/*.ترقيم-ر'),
                    vscode.workspace.createFileSystemWatcher('**/حزمة.toml'),
                    vscode.workspace.createFileSystemWatcher('**/trq.toml'),
                ],
            },
            outputChannel: this.outputChannel,
            traceOutputChannel: this.outputChannel,
            revealOutputChannelOn: RevealOutputChannelOn.Never,
            initializationOptions: {
                locale: locale,
                formatting: {
                    enabled: config.get<boolean>('formatting.enabled', true),
                },
                inlayHints: {
                    enabled: config.get<boolean>('inlayHints.enabled', true),
                    parameterNames: config.get<boolean>('inlayHints.parameterNames', true),
                    variableTypes: config.get<boolean>('inlayHints.variableTypes', true),
                },
                diagnostics: {
                    enabled: config.get<boolean>('diagnostics.enabled', true),
                },
                completion: {
                    enabled: config.get<boolean>('completion.enabled', true),
                },
                hover: {
                    enabled: config.get<boolean>('hover.enabled', true),
                },
            },
            middleware: {
                // Add middleware for request/response logging
                provideCompletionItem: (document, position, context, token, next) => {
                    return next(document, position, context, token);
                },
                provideHover: (document, position, token, next) => {
                    return next(document, position, token);
                },
            },
        };
    }

    /**
     * Resolve language preference
     */
    private resolveLanguage(preference: string): string {
        if (preference === 'auto') {
            // Use VS Code's locale setting
            const vscodeLang = vscode.env.language;
            if (vscodeLang.startsWith('ar')) {
                return 'ar';
            }
            return 'en';
        }
        return preference;
    }

    /**
     * Handle client state changes
     */
    private handleStateChange(state: State): void {
        switch (state) {
            case State.Starting:
                this.log('Language server is starting...');
                this.statusBarItem.text = '$(sync~spin) ترقيم';
                this.statusBarItem.tooltip = 'Tarqeem: Starting... - جاري البدء';
                break;
            case State.Running:
                this.log('Language server is running');
                this.showStatusBar(true);
                break;
            case State.Stopped:
                this.log('Language server stopped');
                this.showStatusBar(false);
                break;
        }
    }

    /**
     * Update status bar based on server state
     */
    private showStatusBar(running: boolean): void {
        if (running) {
            this.statusBarItem.text = '$(check) ترقيم';
            this.statusBarItem.tooltip = 'Tarqeem Language Server: Running - يعمل';
            this.statusBarItem.backgroundColor = undefined;
        } else {
            this.statusBarItem.text = '$(error) ترقيم';
            this.statusBarItem.tooltip = 'Tarqeem Language Server: Stopped - متوقف';
            this.statusBarItem.backgroundColor = new vscode.ThemeColor(
                'statusBarItem.errorBackground'
            );
        }
        this.statusBarItem.show();
    }

    /**
     * Log a message
     */
    private log(message: string): void {
        const timestamp = new Date().toISOString();
        this.outputChannel.appendLine(`[${timestamp}] [Client] ${message}`);
    }
}
