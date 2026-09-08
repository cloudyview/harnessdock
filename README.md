# HarnessDock

**DeepSeek Harness（DSH）实例管理器。** 一个桌面应用，双击即开，用来创建、启动、停止、删除 DSH 实例，统一分配端口，为每个实例安装插件、切换模型，并把配置好的实例捕获成模板批量复制。

> DSH = [`@deepseek-ai/dsh`](https://www.npmjs.com/package/@deepseek-ai/dsh)，DeepSeek 官方的 agent harness。它把"一个实例"定义得很干净：一份运行时 + 一个 `DSH_HOME` + 一个 profile + 一个端口。HarnessDock 就是把这四样东西登记成一张表，然后替你拼启动命令、看进程、改配置。

## 功能

| 页面 | 做什么 |
|---|---|
| **实例** | 一键启动 / 停止 / 重启 / 在浏览器打开；状态、端口、日志一屏看全。启动前探测端口冲突与缺失凭证，失败原因直接写在实例上 |
| **新建向导** | 选模板 → 位置与端口 → 模型与凭证 → 确认。默认位置可配，每项可改 |
| **首次启动 / 导入** | 第一次打开就扫描所有磁盘和 `~/.dsh`，把已有的 dsh 安装收编进来：复用它们的 `node_modules` 当运行时（不重新下载），迁移 home 或原地纳管（patch 引用安装根目录本地包的自动原地），端口重新分配，原目录保留 |
| **模板** | 从任意实例捕获：profile 配置 + 插件清单 + home 级设定 + 基线 `--dump-config`。**永不包含**会话记录与凭证 |
| **端口** | 5 位端口池（默认 41000–42023），可视化占用图，外部进程占用一眼可见 |
| **模型** | 统一维护模型提供方（DeepSeek 官方 / 任意 OpenAI 兼容网关）与默认模型；实例可单独覆盖。应用 = 往实例 patch 文件写两个带标记的区块 |
| **运行时** | 每个 dsh 版本一份 `node_modules`，所有实例共享。升级走金丝雀：新版本跑 dump-config 与当前逐行比对，看过差异再切 |
| **设置** | 实例根目录 / 工作空间根目录 / 运行时目录 / 端口池 / npm 镜像 / 关窗行为（缩托盘或退出） |

## 形态

- **Tauri 2** 桌面应用（Rust 后端 + React 前端），Windows 优先，安装包约 10 MB，使用系统 WebView2。
- 没有需要单独启动的服务进程。关闭窗口默认缩到托盘，实例继续运行。
- 对 dsh 的控制全部是进程级：`node …/dsh/lib/bin.js --profile web --port N --no-open`，解析 stdout 的 `dsh web:` 就绪行；停止用 `taskkill /T` 杀整棵进程树。

## 依赖

- Windows 10/11（WebView2 已内置）或 macOS 12+
- Node.js ≥ 20（运行 dsh）
- pnpm（dsh 用它安装插件；`npm i -g pnpm`）

## 开发

```bash
pnpm install
pnpm tauri dev        # 桌面窗口 + 热更新
pnpm dev              # 仅前端，浏览器演示模式（示例数据，不连真实 dsh）
pnpm typecheck
cd src-tauri && cargo test
pnpm tauri build      # 产出安装包到 src-tauri/target/release/bundle
```

数据目录：`~\.harnessdock\`（`registry.json`、日志、模板、回收站），与 `~\.dsh` 同风格。可用环境变量 `HARNESSDOCK_DATA` 覆盖。安装包（NSIS，按用户安装）把程序放在 `%LOCALAPPDATA%\HarnessDock\`，两者分开，卸载不会动数据；也不放 AppData，避免被沙盒化启动器（MSIX 打包应用）虚拟化后正常启动看不到。

## 与 dsh 的约定

- `cordis.patch.yml` 含 `!!js` 自定义标签，本应用**不整文件解析重写**，只追加 / 替换带 `# harnessdock:begin <key>` … `# harnessdock:end <key>` 标记的区块；你手写的内容不会被动。每次写完可一键跑 `--dump-config` 校验。
- 安装插件必须写明确版本号（禁止 `latest`），因为 npm 上部分 dsh 包的 dist-tag 指向断依赖的旧版。
- dsh 仍是 rc 版，官方明说会破坏兼容；运行时按版本目录隔离，模板绑定版本。

## macOS

源码在 macOS 上可直接构建（Tauri 2 → `.app` + `.dmg`）。与 Windows 的差异都在进程/端口层：
端口占用用 `lsof`、进程存活用 `kill -0`、命令行用 `ps`，停止用 `pkill -P` + `kill`。
从 Finder 启动的 App 没有 shell 的 PATH，所以设置里的 node / pnpm 建议填绝对路径；
App 启动时会把这两个目录补进自己的 PATH，dsh 内部调用 pnpm 才能找到。

## 命令行 `hdock`

同一份代码里还有一个 CLI（`src-tauri/src/bin/hdock.rs`），给脚本和编码 agent（Codex、Claude Code）用。
它与桌面 App 共用 `~/.harnessdock/registry.json`：CLI 写了登记表 App 会自动重读；CLI 启动的实例 App 通过端口扫描接管。
App 不需要开着。

```bash
cargo build --release --bin hdock          # 产物 src-tauri/target/release/hdock，复制到 PATH 即可
hdock list                                 # 实例、状态、端口
hdock scan                                 # 找出磁盘上未纳管的 dsh 安装
hdock import <id> --path <安装目录>          # 原地纳管，端口从池内重新分配
hdock create <id> --template blank-web      # 新建，端口从池内分配
hdock start <id> / stop <id> / restart <id>
hdock validate <id>                        # dsh --dump-config
hdock logs <id> -n 100
hdock plugin add <id> <包>@<版本>
hdock model set <id> deepseek/deepseek-v4-pro
hdock runtime install 0.1.2-rc.1           # 金丝雀：装新版 → 在一个实例上 validate → runtime default
hdock template capture <id> <名称>
```

所有命令支持 `--json`。规则与 App 一致：端口只从端口池分配，没有命令接受手填端口；版本必须写明确版本号。
`hdock delete <id>` 默认只从登记表移除、不动磁盘；`--trash` / `--hard` 才会动 home。

给 Codex 用的 skill 在 `skills/harnessdock/SKILL.md`，复制到 `~/.codex/skills/harnessdock/` 即可。

## 状态

v0.1 开发中。设计原型见 [design/prototype-v1.html](design/prototype-v1.html)。

## License

MIT
