//! Line-level editing of `cordis.patch.yml`.
//!
//! dsh patch files use custom YAML tags (`!!js …`) that generic YAML libraries
//! mangle, so we never parse/re-serialize the whole file. Every block this app
//! writes is fenced with marker comments and can be replaced or removed without
//! touching anything the user wrote by hand.

use crate::model::Provider;

const BEGIN: &str = "# harnessdock:begin ";
const END: &str = "# harnessdock:end ";

/// The shipped placeholder file is literally `[]` (plus comments). A top-level
/// `[]` followed by list items is invalid YAML, so drop it once we add entries.
fn strip_empty_list(text: &str) -> String {
    text.lines()
        .filter(|l| l.trim() != "[]")
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn has_block(text: &str, key: &str) -> bool {
    text.lines().any(|l| l.trim() == format!("{BEGIN}{key}"))
}

pub fn remove_block(text: &str, key: &str) -> String {
    let b = format!("{BEGIN}{key}");
    let e = format!("{END}{key}");
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let t = line.trim();
        if t == b {
            inside = true;
            continue;
        }
        if t == e {
            inside = false;
            continue;
        }
        if !inside {
            out.push(line);
        }
    }
    let mut s = out.join("\n");
    // collapse 3+ blank lines left behind
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    // dsh requires a top-level YAML *array*; an all-comment file parses as null
    // and fails loudly at boot, so restore the shipped `[]` placeholder.
    let has_content = s.lines().any(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with('#')
    });
    if !has_content {
        s = s.trim_end().to_string();
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str("[]\n");
    }
    s
}

pub fn upsert_block(text: &str, key: &str, body: &str) -> String {
    let base = strip_empty_list(&remove_block(text, key));
    let mut s = base.trim_end().to_string();
    if !s.is_empty() {
        s.push_str("\n\n");
    }
    s.push_str(&format!("{BEGIN}{key}\n{}\n{END}{key}\n", body.trim_end()));
    s
}

pub fn list_blocks(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix(BEGIN).map(|s| s.to_string()))
        .collect()
}

/// Row id used inside the composed tree for a plugin insert.
pub fn plugin_row_id(name: &str) -> String {
    let mut s = name
        .trim_start_matches("@deepseek-ai/dsh-")
        .trim_start_matches("./")
        .trim_start_matches("../")
        .to_string();
    for suf in [".mjs", ".js", ".cjs", "/index"] {
        if let Some(x) = s.strip_suffix(suf) {
            s = x.to_string();
        }
    }
    let s: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    format!("hd-{}", s.trim_matches('-'))
}

pub fn plugin_key(name: &str) -> String {
    format!("plugin {name}")
}

pub fn plugin_block(name: &str) -> String {
    format!(
        "- insert:\n    - id: {}\n      name: '{}'",
        plugin_row_id(name),
        name.replace('\'', "''")
    )
}

pub const MODEL_KEY: &str = "model";

pub fn model_block(p: &Provider, model: &str) -> String {
    let mut s = String::new();
    if !p.builtin {
        let ctx = p.context_window.unwrap_or(131_072);
        let max = p.max_tokens.unwrap_or(8_192);
        s.push_str(&format!(
            "- id: llm-pi-ai\n  config:\n    providers:\n      {id}:\n        displayName: {name}\n        apiKeyEnv: {key}\n        api: {api}\n        baseURL: {url}\n        models:\n",
            id = p.id,
            name = yaml_str(&p.name),
            key = p.key_env,
            api = p.api,
            url = p.base_url,
        ));
        for m in &p.models {
            s.push_str(&format!(
                "          - id: {m}\n            name: {n}\n            contextWindow: {ctx}\n            maxTokens: {max}\n",
                n = yaml_str(m)
            ));
        }
    }
    s.push_str(&format!(
        "- id: agent-default-model\n  config:\n    provider: {}\n    model: {}",
        p.id,
        yaml_str(model)
    ));
    s
}

fn yaml_str(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c)) && !s.is_empty() {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED: &str = "# Your patch layer for this dsh profile\n# a top-level YAML array\n[]\n";

    fn prov(builtin: bool) -> Provider {
        Provider {
            id: "sf".into(),
            name: "SiliconFlow".into(),
            api: "openai-completions".into(),
            base_url: "https://api.siliconflow.cn/v1".into(),
            key_env: "SILICONFLOW_API_KEY".into(),
            models: vec!["deepseek-ai/DeepSeek-V3".into()],
            context_window: None,
            max_tokens: None,
            builtin,
        }
    }

    #[test]
    fn upsert_replaces_placeholder_list() {
        let out = upsert_block(SHIPPED, "plugin x", "- insert:\n    - id: x\n      name: './x.mjs'");
        assert!(!out.lines().any(|l| l.trim() == "[]"), "placeholder [] must be removed: {out}");
        assert!(out.contains("# Your patch layer"), "comments kept");
        assert!(has_block(&out, "plugin x"));
        assert_eq!(list_blocks(&out), vec!["plugin x".to_string()]);
    }

    #[test]
    fn upsert_is_idempotent_and_preserves_user_rows() {
        let user = "- id: system-prompt\n  config:\n    persona: hi\n";
        let a = upsert_block(user, MODEL_KEY, &model_block(&prov(false), "deepseek-ai/DeepSeek-V3"));
        let b = upsert_block(&a, MODEL_KEY, &model_block(&prov(false), "other"));
        assert_eq!(b.matches("harnessdock:begin model").count(), 1);
        assert!(b.contains("persona: hi"));
        assert!(b.contains("model: other"));
        assert!(!b.contains("model: deepseek-ai/DeepSeek-V3"), "old default-model row replaced");
        assert!(b.contains("baseURL: https://api.siliconflow.cn/v1"));
    }

    #[test]
    fn remove_block_leaves_rest_intact() {
        let a = upsert_block(SHIPPED, "plugin a", plugin_block("./a.mjs").as_str());
        let b = upsert_block(&a, "plugin b", plugin_block("@deepseek-ai/dsh-tool-web").as_str());
        let c = remove_block(&b, "plugin a");
        assert!(!has_block(&c, "plugin a"));
        assert!(has_block(&c, "plugin b"));
        assert!(c.contains("id: hd-tool-web"));
    }

    #[test]
    fn removing_last_block_restores_empty_array() {
        let a = upsert_block(SHIPPED, MODEL_KEY, &model_block(&prov(true), "deepseek-chat"));
        assert!(!a.lines().any(|l| l.trim() == "[]"));
        let b = remove_block(&a, MODEL_KEY);
        assert!(b.lines().any(|l| l.trim() == "[]"), "placeholder restored: {b}");
        assert!(b.contains("# Your patch layer"));
        // and it round-trips back to a real block
        let c = upsert_block(&b, "plugin x", &plugin_block("./x.mjs"));
        assert!(!c.lines().any(|l| l.trim() == "[]"));
        assert!(has_block(&c, "plugin x"));
    }

    #[test]
    fn builtin_provider_writes_only_default_model() {
        let s = model_block(&prov(true), "deepseek-chat");
        assert!(!s.contains("llm-pi-ai"));
        assert!(s.contains("provider: sf"));
    }

    #[test]
    fn row_ids_are_safe() {
        assert_eq!(plugin_row_id("@deepseek-ai/dsh-subagent-claude-code"), "hd-subagent-claude-code");
        assert_eq!(plugin_row_id("./my-ledger/index.mjs"), "hd-my-ledger");
    }
}
