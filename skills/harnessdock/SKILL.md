---
name: harnessdock
description: 用 hdock 命令行管理本机的 DeepSeek Harness (dsh) 实例：列出、新建、导入、启动、停止、派任务、校验、装插件、切模型。当用户提到 HarnessDock、hdock、dsh 实例、"建一个 dsh"、"启动 company 实例"、"让某个实例去做某事"、"给实例装插件" 时使用。
---

# HarnessDock / hdock

HarnessDock 是桌面 App（/Applications/HarnessDock.app），`hdock` 是它的命令行前端。
两者共用同一份登记表 `~/.harnessdock/registry.json`（可能是指向别处的符号链接，或由 `HARNESSDOCK_DATA` 改写）。
CLI 改了登记表 App 会自动重读；CLI 启动的实例 App 通过端口扫描接管。**App 不需要开着。**

一个 dsh 实例 = 一份共享运行时（dsh 版本）+ 一个 DSH_HOME + 一个 profile + 一个端口。

## 硬规则

1. **端口只从端口池分配**（默认 41000–42023）。没有命令接受手填端口，也不要改 registry.json 绕过。
2. **版本必须写明确版本号**（如 `0.1.1-rc.2`），不接受 latest 或范围。插件 spec 同理。
3. **不要手改** `.runtimes/`、`registry.json`、实例 `out.log`。改配置走 `hdock`，或编辑实例 home 里的
   `profiles/<profile>/cordis.patch.yml` 后跑 `hdock validate <id>`。
4. **凭证不经过 hdock，也不经过你**。不要生成、复制、打印或搬运密钥；缺凭证时报告给用户，让他自己补。
   HarnessDock 只存变量名（如 `DEEPSEEK_API_KEY`），从不存 key 的值。dsh 自己按这个顺序找 key，高者胜：

   | 优先级 | 来源 |
   |---|---|
   | 1 | 继承来的进程环境变量（只读，最高） |
   | 2 | `$DSH_HOME/.credentials.yaml` |
   | 3 | 调用时 cwd 下的 `.env` |
   | 4 | `$DSH_HOME/.env` |

   因为进程环境变量会被继承，**用户在 shell 里 export 一次，所有实例都能用**，新实例不必单独配 `.env`。
   所以新建实例后不要一律叫用户去写 `.env`：先跑 `hdock info <id>` 看 `missing_env` 是否真的缺。
5. `hdock delete <id>` 默认只从登记表移除、**不动磁盘**。`--trash` / `--hard` 会动 home，必须用户明确要求才用。

所有命令支持 `--json`；出错时 `--json` 输出 `{"error": "..."}` 并以非零码退出。

## 先看现状

```bash
hdock list            # 实例、状态、端口、home、缺失凭证
hdock info <id>       # 详情：日志路径、profile 目录、missing_env
hdock settings        # 根目录、端口池、node/pnpm 路径
hdock runtime list    # 已装 dsh 版本，* 为默认
hdock template list
hdock model list
```

## 派任务给实例（核心用法）

```bash
hdock run <id> "<任务描述>" [--profile headless] [--cwd <目录>] [--timeout 900]
```

一条命令进、干完退出，打印实例的最终回答。`--json` 时返回 `output`、`seconds`、`session`、`log`。

**任务会留下完整记录，和用户在界面里手打的会话是同一种东西**：任务原文、运行上下文、可用 skills、
模型生成的标题、助手回答，全部写进 `home/sessions*/<按工作目录编码>/session-<id>/session.jsonl`。
返回值里的 `session` 就是这条记录的 id。

`--cwd` 决定记录归到哪个工作区，缺省用实例登记的 cwd。用户想在 DSH 界面里看到这些记录时，
需要在该实例的界面里把这个目录**添加为工作区**（每个实例一次性操作，之后自动出现）。
如果用户说「界面里看不到」，先确认他选的工作区和任务的 cwd 是同一个目录。

`--profile` 缺省 `headless`。若实例没有该 profile，命令会报错并列出现有 profile。

## 新建实例

```bash
hdock create <id> [--template blank-web|blank-headless|<已捕获模板>] [--runtime 0.1.1-rc.2] \
                  [--home <DSH_HOME>] [--cwd <工作目录>] [--model deepseek/deepseek-v4-pro] [--display 名称]
```

- id 只能小写字母、数字、连字符。
- 缺省 home = `<instRoot>/<id>/home`，缺省 cwd = `<wsRoot>/<id>`。**这两个根目录因机器而异，
  动手前先跑 `hdock settings` 看这台机器实际配的是什么**，不要假定。若两者指向同一个目录，
  新实例就落成该目录下的 `<id>/`。
- create 会初始化 profile、装 bundle 插件、跑一次 dump-config，可能要几十秒。
- 建完先跑 `hdock info <id>` 看 `missing_env`：为空就能直接启动；确实缺才请用户补凭证。

**搭架子的分工**：hdock 只管到实例边界。实例内部的 `AGENTS.md`、`skills/`、`.agent-presets/`、
`profiles/<p>/cordis.patch.yml` 都是普通文件，直接用你自己的文件工具写，写完跑 `hdock validate <id>` 验证。

## 导入已有安装

```bash
hdock scan                                    # 找出磁盘上未纳管的 dsh 安装
hdock import <id> --path <安装目录> [--home <DSH_HOME>] [--profile web] [--runtime <版本>] [--migrate]
```

默认原地纳管：home 留在原处（通常在它自己的 git 仓里），只登记。端口按规则重新从池内分配。
`--migrate` 会把 home 复制进 instRoot，一般不要用。

## 启停与排错

```bash
hdock start <id> [--timeout 90]   # 起 web 服务器，等到就绪行，打印 URL
hdock stop <id>
hdock restart <id>
hdock validate <id>               # dsh --dump-config；ok=false 时看 output 末尾
hdock logs <id> -n 100
```

启动失败常见原因：端口被非 dsh 进程占用、`home/.env` 缺凭证、patch 语法错。先看 `hdock logs`。

## 插件 / 模型 / 模板 / 运行时

```bash
hdock plugin list <id>
hdock plugin add <id> <包名>@<明确版本>
hdock plugin remove <id> <包名>
hdock model set <id> <provider>/<model>       # 实例级覆盖；传 "" 回到默认
hdock model default <provider>/<model>
hdock template capture <id> <名称> [--home]   # 永不包含会话与凭证
hdock runtime install <版本>                   # 金丝雀：装新版 → 在一个实例上 validate 比对 → runtime default
hdock runtime default <版本>
```

## 目录约定

先用 `hdock settings` 读出这台机器的 `instRoot` / `wsRoot` / `rtRoot`，再按下面的分工放东西：

- **对话、任务、状态**：实例 `home/`（sessions、storages、attachments），dsh 自动展开。
- **实例专属资产**（skills、总账、agent 预设）：实例 `home/` 下手放。
- **项目代码与产出**：不要放进 home。放在与实例平级的独立目录，在 dsh 里作为工作区打开。
- 运行时目录与登记表：可重建，不入库，不要手改。

如果这台机器另有约定（实例命名、目录布局、端口安排），一般写在 `instRoot` 附近的项目文档里，先看一眼再动手。
