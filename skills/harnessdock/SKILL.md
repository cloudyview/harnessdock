---
name: harnessdock
description: 用 hdock 命令行管理本机的 DeepSeek Harness (dsh) 实例：列出、新建、导入、启动、停止、校验、装插件、切模型。当用户提到 HarnessDock、hdock、dsh 实例、"建一个 dsh"、"启动 company 实例"、"给实例装插件" 时使用。
---

# HarnessDock / hdock

HarnessDock 是桌面 App（/Applications/HarnessDock.app），`hdock` 是它的命令行前端，两者共用同一份登记表
`~/.harnessdock/registry.json`（本机实际位于 /Volumes/DATA/DeepSeekHarnessProject/.harnessdock/）。
CLI 改了登记表 GUI 会自动重读；CLI 启动的实例 GUI 会通过端口扫描接管。GUI 不必开着。

一个 dsh 实例 = 一份共享运行时（dsh 版本）+ 一个 DSH_HOME + 一个 profile + 一个端口。

## 硬规则

1. **端口只从端口池分配**（默认 41000–42023）。没有任何命令接受手填端口；不要去改 registry.json 绕过。
2. **运行时版本必须写明确版本号**（如 `0.1.1-rc.2`），不接受 latest 或范围。插件 spec 同理必须带版本。
3. **不要手改** `.runtimes/`、`registry.json`、实例 `out.log`。改配置走 `hdock`，或直接编辑实例 home 里的
   `profiles/<profile>/cordis.patch.yml` 后跑 `hdock validate <id>`。
4. **凭证不经过 hdock**。API key 写在实例 `home/.env`（如 `DEEPSEEK_API_KEY=...`），由用户自己放；agent 不要生成或搬运密钥。
5. 删除实例默认只是「从登记表移除」，磁盘不动。`--trash` / `--hard` 会动 home，须用户明确要求。

## 先看现状

```bash
hdock list            # 实例、状态、端口、home
hdock settings        # 根目录、端口池、node/pnpm 路径
hdock runtime list    # 已装的 dsh 版本，* 为默认
hdock template list
hdock model list
```
所有命令加 `--json` 得到机器可读输出；出错时 `--json` 模式输出 `{"error": "..."}` 并以非零退出。

## 新建实例

```bash
hdock create <id> [--template blank-web|blank-headless|<已捕获模板>] [--runtime 0.1.1-rc.2] \
                  [--home <DSH_HOME>] [--cwd <工作目录>] [--model deepseek/deepseek-v4-pro] [--display 显示名]
```
- id 只能小写字母、数字、连字符。本机约定实例 id 用 `dsh-<业务名>`。
- 缺省 home = `<instRoot>/<id>/home`，缺省 cwd = `<wsRoot>/<id>`；本机两者都指向
  /Volumes/DATA/DeepSeekHarnessProject，所以新实例会成为该目录下的 `dsh-<name>/`。
- create 会初始化 profile、安装 bundle 插件、跑一次 dump-config；耗时可达数十秒。
- 完成后提醒用户把凭证写进 `<home>/.env`，再 `hdock start <id>`。

## 导入已有 dsh 安装

```bash
hdock scan                                   # 找出磁盘上未纳管的安装
hdock import <id> --path <安装目录> [--home <DSH_HOME>] [--profile web] [--runtime <版本>] [--migrate]
```
默认原地纳管（home 留在原处，只登记）。端口按规则重新从池内分配，旧脚本里的端口号作废。

## 启停与排错

```bash
hdock start <id> [--timeout 90]   # 等到 `dsh web:` 就绪行，打印 URL
hdock stop <id>
hdock restart <id>
hdock validate <id>               # dsh --dump-config；ok=false 时看 output 末尾
hdock logs <id> -n 100
hdock info <id>                   # 含 log 路径、profileDir、缺少的凭证 missing_env
```
启动失败先看 `hdock logs`，常见原因：端口被非 dsh 进程占用、home/.env 缺凭证、patch 语法错。

## 插件 / 模型 / 模板 / 运行时

```bash
hdock plugin list <id>
hdock plugin add <id> <包名>@<明确版本>
hdock plugin remove <id> <包名>
hdock model set <id> <provider>/<model>    # 实例级覆盖；传 "" 回到默认
hdock model default <provider>/<model>
hdock template capture <id> <名称> [--home] # 永不包含会话与凭证
hdock runtime install <版本>                # 新版本先装、在一个实例上 validate 比对，再 runtime default
hdock runtime default <版本>
```

## 目录约定（本机）

- 对话、任务、状态：实例 `home/`（sessions、storages、attachments）由 dsh 自动展开。
- 项目代码：不放进 home，放在与实例平级的独立目录（如 company-biz/），在 dsh 界面里作为工作区打开。
- 共享运行时 `.runtimes/<版本>/`、登记表 `.harnessdock/`，均可重建，不入库。
