use std::collections::HashMap;
use std::sync::OnceLock;

use include_dir::{include_dir, Dir};

static PREPENDS_DIR: Dir<'static> = include_dir!("prepends");

fn prepends_table() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut m = HashMap::new();
        for file in PREPENDS_DIR.files() {
            let Some(path_str) = file.path().to_str() else {
                continue;
            };
            if !path_str.ends_with(".typ") {
                continue;
            }
            let id = path_str[..path_str.len() - 4].replace('\\', "/");
            let text = file
                .contents_utf8()
                .unwrap_or_else(|| panic!("prepends/{path_str} is not valid UTF-8"))
                .to_string();
            if m.insert(id.clone(), text).is_some() {
                panic!("duplicate prepend plugin id after normalize: {id}");
            }
        }
        m
    })
}

pub fn list_embedded_plugin_ids() -> Vec<String> {
    let mut v: Vec<String> = prepends_table().keys().cloned().collect();
    v.sort();
    v
}

pub fn embedded_prepend_source(id: &str) -> Result<String, String> {
    let id = id.trim().replace('\\', "/");
    if id.is_empty() {
        return Err("empty plugin id".into());
    }
    prepends_table()
        .get(&id)
        .cloned()
        .ok_or_else(|| {
            let known = list_embedded_plugin_ids().join(", ");
            if known.is_empty() {
                format!("unknown plugin '{id}' (no embedded prepends in this build)")
            } else {
                format!("unknown plugin '{id}' (embedded: {known})")
            }
        })
}

pub fn concat_plugin_sources(plugin_ids: &[impl AsRef<str>]) -> Result<String, String> {
    let mut out = String::new();
    for (i, id) in plugin_ids.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&embedded_prepend_source(id.as_ref())?);
    }
    Ok(out)
}
