import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    Executable,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
    // 确定平台和可执行文件名
    const platform = process.platform === 'win32' ? 'windows' : 'linux';
    const executableName = process.platform === 'win32' ? 'language_server.exe' : 'language_server';
    
    // 固定路径：editor/code/server/{platform}/{debug|release}/language_server
    const debugServerPath = path.join(context.extensionPath, 'server', platform, 'debug', executableName);
    const releaseServerPath = path.join(context.extensionPath, 'server', platform, 'release', executableName);

    // 配置服务器选项
    const run: Executable = {
        command: releaseServerPath,
        options: {
            env: {
                ...process.env,
                RUST_LOG: 'info',
            },
        },
    };

    const debug: Executable = {
        command: debugServerPath,
        options: {
            env: {
                ...process.env,
                RUST_LOG: 'debug',
                RUST_BACKTRACE: '1',
            },
        },
    };

    const serverOptions: ServerOptions = {
        run,
        debug,
    };

    // 配置客户端选项
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'airyc' },
        ],
        synchronize: {
            // 监听 .airy 文件的变化
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.airy'),
        },
    };

    // 创建语言客户端
    client = new LanguageClient(
        'airycLanguageServer',
        'Airyc Language Server',
        serverOptions,
        clientOptions
    );

    // 启动客户端（这也会启动服务器）
    client.start().catch((error) => {
        vscode.window.showErrorMessage(
            `Failed to start Airyc language server: ${error.message}\n\n` +
            `Please ensure the language server is built. Run:\n` +
            `npm run build:server:release`
        );
    });

    // 注册重启服务器命令
    context.subscriptions.push(
        vscode.commands.registerCommand('airyc.restartServer', async () => {
            if (client) {
                await client.stop();
                await client.start();
                vscode.window.showInformationMessage('Airyc language server restarted.');
            }
        })
    );
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
