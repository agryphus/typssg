mod plugin;

use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

pub use plugin::{concat_plugin_sources, embedded_prepend_source, list_embedded_plugin_ids};

use typst::ecow::EcoString;
use typst::syntax::Source;
use typst_as_lib::{typst_kit_options::TypstKitFontOptions, TypstAsLibError, TypstEngine};
use typst_html::{HtmlAttr, HtmlDocument, HtmlElement, HtmlNode};
use log::info;

fn format_typst_compile_error(
    err: TypstAsLibError,
    full_source: &str,
    index_byte_start: usize,
    index_source: &str,
) -> std::io::Error {
    let report = match err {
        TypstAsLibError::TypstSource(diagnostics) if !diagnostics.is_empty() => {
            let combined = Source::detached(full_source);
            let index_only = Source::detached(index_source);
            let index_end = index_byte_start.saturating_add(index_source.len());
            let mut out = String::from("Typst compile failed:\n");
            for d in diagnostics.iter() {
                let msg = d.message.as_str();
                if let Some(range) = combined.range(d.span) {
                    let byte = range.start;
                    if byte >= index_byte_start && byte < index_end {
                        let rel = byte - index_byte_start;
                        if let Some((line, col)) = index_only.lines().byte_to_line_column(rel) {
                            let _ = writeln!(
                                &mut out,
                                "  index.typ:{}:{}: {}",
                                line + 1,
                                col + 1,
                                msg
                            );
                        } else {
                            let _ = writeln!(&mut out, "  {msg}");
                        }
                    } else if let Some((line, col)) = combined.lines().byte_to_line_column(byte) {
                        let _ = writeln!(
                            &mut out,
                            "  (preamble) line {}:{}: {}",
                            line + 1,
                            col + 1,
                            msg
                        );
                    } else {
                        let _ = writeln!(&mut out, "  {msg}");
                    }
                } else {
                    let _ = writeln!(&mut out, "  {msg}");
                }
                for hint in d.hints.iter() {
                    let _ = writeln!(&mut out, "    hint: {}", hint.as_str());
                }
            }
            out
        }
        other => format!("typst compile failed: {other}"),
    };
    std::io::Error::new(std::io::ErrorKind::Other, report)
}


pub fn compile_article(
    article_dir: &PathBuf,
    prepend: &Option<PathBuf>,
    plugins: &[impl AsRef<str>],
    include_title: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("compiling {} ...", article_dir.display());

    let template_file = article_dir.join("index.typ");
    let output = article_dir.join("index.html");
    let outline_file = article_dir.join("outline.html");

    let plugin_block = concat_plugin_sources(plugins)?;

    let user_prepend = if let Some(prepend_file) = prepend {
        fs::read_to_string(&prepend_file).map_err(|e| {
            format!(
                "could not read prepend file {}: {e}",
                prepend_file.display()
            )
        })?
    } else {
        fs::read_to_string(article_dir.join("prepend.typ")).unwrap_or_default()
    };

    let index_source = fs::read_to_string(&template_file).map_err(|e| {
        format!(
            "could not read template {}: {e}",
            template_file.display()
        )
    })?;

    let mut template: EcoString = EcoString::new();
    template.push_str(&plugin_block);
    if !plugin_block.is_empty() && !user_prepend.is_empty() {
        template.push('\n');
    }
    template.push_str(&user_prepend);
    let index_byte_start = template.len();
    template.push_str(&index_source);

    let full_source_str = template.to_string();

    let engine = TypstEngine::builder()
        .main_file(full_source_str.clone())
        .search_fonts_with(
            TypstKitFontOptions::default()
                .include_system_fonts(false)
                .include_embedded_fonts(true),
        )
        .with_file_system_resolver(article_dir)
        .build();

    let mut doc: HtmlDocument = engine
        .compile()
        .output
        .map_err(|e| format_typst_compile_error(e, &full_source_str, index_byte_start, &index_source))?;

    let mut outline = EcoString::new();
    let mut curr_level = 1u32;
    let mut ul_depth = 0u32;
    let mut title_h2_pending = !include_title;
    let mut first_outline_heading = true;
    parse_outline(
        &mut doc.root,
        &mut outline,
        &mut curr_level,
        &mut ul_depth,
        include_title,
        &mut title_h2_pending,
        &mut first_outline_heading,
    );
    // `ul_depth` counts open `<ul>` tags; it can diverge from `curr_level - 1` when the first
    // outline heading uses lazy depth (skip title). Always drain by `ul_depth`, not `curr_level`.
    while ul_depth > 0 {
        ul_depth -= 1;
        outline.push_str("  ".repeat(ul_depth as usize).as_str());
        outline.push_str("</ul>\n");
    }
    fs::write(&outline_file, outline.as_bytes()).map_err(|e| {
        format!(
            "could not write outline {}: {e}",
            outline_file.display()
        )
    })?;

    let mut body: Option<HtmlElement> = None;
    for child in &doc.root.children {
        match child {
            HtmlNode::Element(e) if e.tag.to_string().as_str() == "<body>" => {
                body = Some(e.clone());
            }
            _ => {}
        }
    }
    let body = body.ok_or("compiled HTML has no <body> element")?;
    doc.root = body;

    let mut html: String = typst_html::html(&doc).map_err(|e| format!("html generation failed: {e:?}"))?;
    let lines = html
        .lines()
        .map(|line| {
            if line.len() >= 2 {
                &line[2..]
            } else {
                ""
            }
        })
        .collect::<Vec<&str>>();

    if lines.len() > 2 {
        html = lines[2..lines.len() - 1].to_vec().join("\n");
    } else {
        html = String::new();
    }

    fs::write(&output, html).map_err(|e| {
        format!(
            "could not write output {}: {e}",
            output.display()
        )
    })?;

    Ok(())
}

fn heading_level_from_tag(tag: &str) -> Option<u32> {
    match tag {
        "<h2>" => Some(2),
        "<h3>" => Some(3),
        "<h4>" => Some(4),
        "<h5>" => Some(5),
        "<h6>" => Some(6),
        _ => None,
    }
}

fn parse_outline(
    elem: &mut HtmlElement,
    outline: &mut EcoString,
    curr_level: &mut u32,
    ul_depth: &mut u32,
    include_title: bool,
    title_h2_pending: &mut bool,
    first_outline_heading: &mut bool,
) {
    let tag = elem.tag.to_string();
    let tag_ref = tag.as_str();

    if let Some(level) = heading_level_from_tag(tag_ref) {
        if !include_title && *title_h2_pending {
            *title_h2_pending = false;
            if tag_ref == "<h2>" {
                for child in elem.children.make_mut().iter_mut() {
                    if let HtmlNode::Element(e) = child {
                        parse_outline(
                            e,
                            outline,
                            curr_level,
                            ul_depth,
                            include_title,
                            title_h2_pending,
                            first_outline_heading,
                        );
                    }
                }
                return;
            }
        }
        *title_h2_pending = false;

        let mut header_text = EcoString::new();

        for child in &elem.children {
            match child {
                HtmlNode::Text(string, _) => {
                    header_text.push_str(string);
                }
                _ => {}
            }
        }

        let slug = header_text
            .as_str()
            .to_lowercase()
            .chars()
            .filter_map(|c| match c {
                c if c.is_ascii_alphanumeric() => Some(c),
                ' ' => Some('-'),
                '\'' => None,
                _ => None,
            })
            .collect::<String>()
            .trim_matches('-')
            .replace("--", "-");

        if *first_outline_heading {
            *first_outline_heading = false;
            *curr_level = level.saturating_sub(1);
        }

        while level > *curr_level {
            outline.push_str("  ".repeat(*ul_depth as usize).as_str());
            outline.push_str("<ul>\n");
            *ul_depth += 1;
            *curr_level += 1;
        }
        while level < *curr_level && *ul_depth > 0 {
            *curr_level -= 1;
            *ul_depth -= 1;
            outline.push_str("  ".repeat(*ul_depth as usize).as_str());
            outline.push_str("</ul>\n");
        }
        *curr_level = level;

        outline.push_str("  ".repeat(*ul_depth as usize).as_str());
        outline.push_str(
            format!(
                "<li><a href=\"#{}\">{}</a></li>\n",
                slug, header_text
            )
            .as_str(),
        );

        elem.attrs.push(HtmlAttr::intern("id").unwrap(), slug);
        return;
    }

    for child in elem.children.make_mut().iter_mut() {
        match child {
            HtmlNode::Element(e) => {
                parse_outline(
                    e,
                    outline,
                    curr_level,
                    ul_depth,
                    include_title,
                    title_h2_pending,
                    first_outline_heading,
                );
            }
            _ => {}
        }
    }
}

pub fn compile_all(
    root_dir: &PathBuf,
    prepend: &Option<PathBuf>,
    plugins: &[impl AsRef<str>],
    include_title_in_outline: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for entry in fs::read_dir(root_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            compile_all(&path, prepend, plugins, include_title_in_outline)?;
        } else if path.file_name().is_some_and(|n| n == "index.typ") {
            let dir = path.parent().unwrap().to_path_buf();
            compile_article(&dir, prepend, plugins, include_title_in_outline)?;
        }
    }

    Ok(())
}
