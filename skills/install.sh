#!/bin/sh
# 从唯一规范源 skills/harnessdock/SKILL.md 生成三份 skill 并安装。
# 正文只有一份，改内容改规范源后重跑本脚本；不要分别编辑安装出去的副本。
set -eu
SRC="$(cd "$(dirname "$0")" && pwd)/harnessdock/SKILL.md"
[ -f "$SRC" ] || { echo "找不到规范源 $SRC" >&2; exit 1; }

install_one() {
  root="$1"; agent="$2"; note="$3"
  dir="$root/harnessdock"
  [ -d "$root" ] || { echo "跳过 $agent：$root 不存在"; return 0; }
  mkdir -p "$dir"
  {
    # 原样搬运 frontmatter 与正文，只在标题后插一段该 agent 的定位
    awk -v note="$note" '
      /^# HarnessDock \/ hdock$/ { print; print ""; print note; next }
      { print }
    ' "$SRC"
  } > "$dir/SKILL.md"
  echo "已安装 $agent → $dir/SKILL.md"
}

install_one "$HOME/.codex/skills" "Codex" \
"> **在 Codex 里的定位**：你是搭架子的人。用 hdock 建实例、装插件、配模型，用你自己的文件工具写实例内部的 AGENTS.md 与 skills，最后用 \`hdock run\` 把任务交给实例自己跑。代码 review 之类的事你直接做，不必绕经 hdock。"

install_one "$HOME/.claude/skills" "Claude Code" \
"> **在 Claude Code 里的定位**：你既搭架子也排错。建实例与写实例内容同下；实例出问题时先 \`hdock validate\` 和 \`hdock logs\`，再去改 profile 的 patch。需要让某个实例代跑一件事时用 \`hdock run\`。"

install_one "$HOME/.workbuddy/skills" "WorkBuddy" \
"> **在 WorkBuddy 里的定位**：你偏调度而非建造。日常主要用 \`hdock list\` 看实例状态、\`hdock run\` 把业务任务派给对应实例、\`hdock logs\` 回看。新建与改造实例前先跟用户确认。"
