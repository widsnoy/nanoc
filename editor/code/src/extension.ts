import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    Executable,
    Trace,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

const traceMap: { [key: string]: Trace } = {
    'off': Trace.Off,
    'messages': Trace.Messages,
    'compact': Trace.Compact,
    'verbose': Trace.Verbose,
};

function getTraceLevel(): Trace {
    const traceSettingValue = vscode.workspace
        .getConfiguration('airyc')
        .get<string>('trace.server') || 'off';
    return traceMap[traceSettingValue] || Trace.Off;
}

export function activate(context: vscode.ExtensionContext) {
    // 创建输出通道记录扩展信息
    const outputChannel = vscode.window.createOutputChannel('airyc');
    outputChannel.appendLine('Airyc extension activating...');

    // 确定平台和可执行文件名
    const platform = process.platform === 'win32' ? 'windows' : 'linux';
    const executableName = process.platform === 'win32' ? 'airyc-server.exe' : 'airyc-server';

    // 固定路径：editor/code/server/{platform}/{debug|release}/language_server
    const debugServerPath = path.join(context.extensionPath, 'server', platform, 'debug', executableName);
    const releaseServerPath = path.join(context.extensionPath, 'server', platform, 'release', executableName);

    // RUST_LOG 可以在 launch.json 中设置
    const logLevel = process.env.RUST_LOG || 'info';
    outputChannel.appendLine(`Log level: ${logLevel}`);

    // 配置服务器选项
    const run: Executable = {
        command: releaseServerPath,
        options: {
            env: {
                ...process.env,
                RUST_LOG: logLevel,
            },
        },
    };

    const debug: Executable = {
        command: debugServerPath,
        options: {
            env: {
                ...process.env,
                RUST_LOG: logLevel,
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
        outputChannel: outputChannel,
    };

    // 创建语言客户端
    client = new LanguageClient(
        'airyc',
        'airyc-lsp',
        serverOptions,
        clientOptions
    );

    // 启动客户端（这也会启动服务器）
    client.start().then(() => {
        outputChannel.appendLine('Language server started successfully');

        // 关键修复：在 start() 之后调用 setTrace()
        const traceLevel = getTraceLevel();
        client!.setTrace(traceLevel);
        outputChannel.appendLine(`Trace level set to: ${traceLevel}`);

        outputChannel.show(true);
    }).catch((error) => {
        outputChannel.appendLine(`Error starting server: ${error.message}`);
        vscode.window.showErrorMessage(
            `Failed to start airyc language server: ${error.message}`
        );
    });

    // 监听配置变化，动态更新 trace 级别
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('airyc.trace') && client) {
                const newTraceLevel = getTraceLevel();
                client.setTrace(newTraceLevel);
                outputChannel.appendLine(`Trace level changed to: ${newTraceLevel}`);
            }
        })
    );

    // 注册重启服务器命令
    context.subscriptions.push(
        vscode.commands.registerCommand('airyc.restartServer', async () => {
            if (client) {
                await client.stop();
                await client.start();
                client.setTrace(getTraceLevel());
                vscode.window.showInformationMessage('airyc server restarted.');
            }
        })
    );

    context.subscriptions.push(outputChannel);
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
