use std::fs;
use std::path::PathBuf;

use typst::ecow::EcoString;
use typst_as_lib::{typst_kit_options::TypstKitFontOptions, TypstEngine};
use typst_html::{HtmlAttr, HtmlDocument, HtmlElement, HtmlNode};


pub fn compile_article(
    article_dir: &PathBuf,
    prepend: &Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let template_file = article_dir.join("index.typ");
    let output = article_dir.join("index.html");
    let outline_file = article_dir.join("outline.html");

    let prepend_content = if let Some(prepend_file) = prepend {
        fs::read_to_string(&prepend_file).map_err(|e| {
            format!(
                "could not read prepend file {}: {e}",
                prepend_file.display()
            )
        })?
    } else {
        fs::read_to_string(article_dir.join("prepend.typ")).unwrap_or_default()
    };

    let mut template: EcoString = EcoString::new();
    template.push_str(&prepend_content);
    template.push_str(
        &fs::read_to_string(&template_file).map_err(|e| {
            format!(
                "could not read template {}: {e}",
                template_file.display()
            )
        })?,
    );

    let engine = TypstEngine::builder()
        .main_file(template.to_string())
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
        .map_err(|e| format!("typst compile failed: {e}"))?;

    let mut outline = EcoString::new();
    let mut curr_level = 1;
    parse_outline(&mut doc.root, &mut outline, &mut curr_level);
    for i in (1..curr_level).rev() {
        outline.push_str("  ".repeat(i as usize - 1).as_str());
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

fn parse_outline(elem: &mut HtmlElement, outline: &mut EcoString, curr_level: &mut u32) {
    if matches!(
        elem.tag.to_string().as_str(),
        "<h2>" | "<h3>" | "<h4>" | "<h5>" | "<h6>"
    ) {
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

        let level: u32 = elem.tag.to_string().chars().nth(2).unwrap() as u32 - '0' as u32;

        while level > *curr_level {
            *curr_level += 1;
            outline.push_str("  ".repeat(*curr_level as usize - 2).as_str());
            outline.push_str("<ul>\n");
        }
        while level < *curr_level {
            outline.push_str("  ".repeat(*curr_level as usize - 2).as_str());
            outline.push_str("</ul>\n");
            *curr_level -= 1;
        }
        *curr_level = level;

        outline.push_str("  ".repeat(*curr_level as usize - 1).as_str());
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
            HtmlNode::Element(e) => parse_outline(e, outline, curr_level),
            _ => {}
        }
    }
}
