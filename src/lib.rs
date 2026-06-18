mod plugin;

use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

pub use plugin::{concat_plugin_sources, embedded_prepend_source, list_embedded_plugin_ids};

use log::{error, info};
use typst::diag::{SourceDiagnostic, Warned};
use typst::ecow::EcoString;
use typst::foundations::{Datetime, Duration};
use typst::syntax::{DiagSpanKind, FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use typst::{LibraryExt, World};
use typst_html::{HtmlAttr, HtmlDocument, HtmlElement, HtmlNode, HtmlOptions};

struct TypssgWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    main_source_id: FileId,
    main_source: Source,
    root: PathBuf,
    fonts: Vec<Font>,
}

impl World for TypssgWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_source_id
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == self.main_source_id {
            Ok(self.main_source.clone())
        } else {
            let path = id.vpath().realize(&self.root).ok();
            if let Some(path) = path {
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| typst::diag::FileError::from_io(e, &path))?;
                Ok(Source::new(id, text))
            } else {
                Err(typst::diag::FileError::NotFound(self.root.clone()))
            }
        }
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<typst::foundations::Bytes> {
        if id == self.main_source_id {
            Ok(typst::foundations::Bytes::from_string(
                self.main_source.text().to_owned(),
            ))
        } else {
            let path = id.vpath().realize(&self.root).ok();
            if let Some(path) = path {
                let data =
                    std::fs::read(&path).map_err(|e| typst::diag::FileError::from_io(e, &path))?;
                Ok(typst::foundations::Bytes::new(data))
            } else {
                Err(typst::diag::FileError::NotFound(self.root.clone()))
            }
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let mut now = time::OffsetDateTime::now_utc();
        if let Some(offset) = offset {
            let td: time::Duration = offset.into();
            now = now.checked_add(td)?;
        }
        Datetime::from_ymd(now.year(), now.month() as u8, now.day())
    }
}

fn article_main_vpath(article_dir: &Path, root_dir: &Path) -> String {
    let article = fs::canonicalize(article_dir).unwrap_or_else(|_| article_dir.to_path_buf());
    let root = fs::canonicalize(root_dir).unwrap_or_else(|_| root_dir.to_path_buf());
    let rel = article.strip_prefix(&root).unwrap_or(article.as_path());
    rel.join("index.typ").to_string_lossy().replace('\\', "/")
}

fn format_typst_compile_error(
    diagnostics: Vec<SourceDiagnostic>,
    full_source: &str,
    index_byte_start: usize,
    index_source: &str,
) -> std::io::Error {
    let combined = Source::detached(full_source);
    let index_only = Source::detached(index_source);
    let index_end = index_byte_start.saturating_add(index_source.len());
    let mut out = String::from("Typst compile failed:\n");
    for d in &diagnostics {
        let msg = d.message.as_str();
        let range = match d.span.get() {
            DiagSpanKind::Number { num, sub_range, .. } => combined.range(num, sub_range),
            DiagSpanKind::Range { range, .. } => Some(range),
            DiagSpanKind::Detached => None,
        };
        if let Some(range) = range {
            let byte = range.start;
            if byte >= index_byte_start && byte < index_end {
                let rel = byte - index_byte_start;
                if let Some((line, col)) = index_only.lines().byte_to_line_column(rel) {
                    let _ = writeln!(&mut out, "  index.typ:{}:{}: {}", line + 1, col + 1, msg);
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
        for hint in &d.hints {
            let _ = writeln!(&mut out, "    hint: {}", hint.v.as_str());
        }
    }
    std::io::Error::other(out)
}

/// Internal compilation kernel. `index_byte_start` is the byte offset within
/// `full_source` where the user's source begins (plugins/prepend precede it).
fn compile_source(
    full_source: &str,
    index_byte_start: usize,
    vfs_root: &PathBuf,
    main_vpath: &str,
    include_title: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let index_source = &full_source[index_byte_start..];

    let main_source_id = RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(main_vpath).map_err(|_| format!("invalid virtual path: {main_vpath}"))?,
    )
    .intern();
    let mut book = FontBook::new();
    let mut fonts = Vec::new();
    for data in typst_assets::fonts() {
        if let Some(font) = Font::new(typst::foundations::Bytes::new(data), 0) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }

    let world = TypssgWorld {
        library: LazyHash::new(
            Library::builder()
                .with_features([typst::Feature::Html].into_iter().collect())
                .build(),
        ),
        book: LazyHash::new(book),
        main_source_id,
        main_source: Source::new(main_source_id, full_source.to_string()),
        root: vfs_root.clone(),
        fonts,
    };

    let Warned { output, .. } = typst::compile::<HtmlDocument>(&world);
    let mut doc = output.map_err(|e| {
        format_typst_compile_error(e.to_vec(), full_source, index_byte_start, index_source)
    })?;

    let mut outline = EcoString::new();
    let mut curr_level = 1u32;
    let mut ul_depth = 0u32;
    let mut title_h2_pending = !include_title;
    let mut first_outline_heading = true;
    parse_outline(
        doc.root_mut(),
        &mut outline,
        &mut curr_level,
        &mut ul_depth,
        include_title,
        &mut title_h2_pending,
        &mut first_outline_heading,
    );
    while ul_depth > 0 {
        ul_depth -= 1;
        outline.push_str("  ".repeat(ul_depth as usize).as_str());
        outline.push_str("</ul>\n");
    }
    let outline_str = outline.to_string();

    let body: HtmlElement = {
        let mut body: Option<HtmlElement> = None;
        for child in &doc.root().children {
            match child {
                HtmlNode::Element(e) if e.tag.to_string().as_str() == "<body>" => {
                    body = Some(e.clone());
                }
                _ => {}
            }
        }
        body.ok_or("compiled HTML has no <body> element")?
    };

    *doc.root_mut() = body;

    let mut html: String =
        typst_html::html(&doc, &HtmlOptions { pretty: true })
            .map_err(|e| format!("html generation failed: {e:?}"))?;
    let lines = html
        .lines()
        .map(|line| if line.len() >= 2 { &line[2..] } else { "" })
        .collect::<Vec<&str>>();

    if lines.len() > 2 {
        html = lines[2..lines.len() - 1].to_vec().join("\n");
    } else {
        html = String::new();
    }

    Ok((html, outline_str))
}

/// Render Typst source directly from a string. Plugins are prepended
/// automatically. `base_dir` is used by the Typst engine for file-system
/// resolution (images, includes, etc.). Returns `(index_html, outline_html)`.
pub fn render_direct(
    source: &str,
    plugins: &[impl AsRef<str>],
    include_title: bool,
    base_dir: &PathBuf,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let plugin_block = concat_plugin_sources(plugins)?;

    let mut full_source = EcoString::new();
    full_source.push_str(&plugin_block);
    if !plugin_block.is_empty() {
        full_source.push('\n');
    }
    let index_byte_start = full_source.len();
    full_source.push_str(source);

    compile_source(
        full_source.as_str(),
        index_byte_start,
        base_dir,
        "index.typ",
        include_title,
    )
}

/// Render Typst source read from `article_dir/index.typ`. Supports an
/// optional `prepend` file and writes the output to `index.html` and
/// `outline.html` in the same directory.
pub fn compile_article(
    article_dir: &PathBuf,
    root_dir: &PathBuf,
    prepend: &Option<PathBuf>,
    plugins: &[impl AsRef<str>],
    include_title: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("compiling {} ...", article_dir.display());

    let output = article_dir.join("index.html");
    let outline_file = article_dir.join("outline.html");

    let plugin_block = concat_plugin_sources(plugins)?;
    let main_vpath = article_main_vpath(article_dir, root_dir);

    let user_prepend = if let Some(prepend_file) = prepend {
        fs::read_to_string(prepend_file).map_err(|e| {
            format!(
                "could not read prepend file {}: {e}",
                prepend_file.display()
            )
        })?
    } else {
        fs::read_to_string(article_dir.join("prepend.typ")).unwrap_or_default()
    };

    let template_file = article_dir.join("index.typ");
    let index_source = fs::read_to_string(&template_file)
        .map_err(|e| format!("could not read template {}: {e}", template_file.display()))?;

    let mut full_source: EcoString = EcoString::new();
    full_source.push_str(&plugin_block);
    if !plugin_block.is_empty() && !user_prepend.is_empty() {
        full_source.push('\n');
    }
    full_source.push_str(&user_prepend);
    let index_byte_start = full_source.len();
    full_source.push_str(&index_source);

    let (index_html, outline_html) = compile_source(
        full_source.as_str(),
        index_byte_start,
        root_dir,
        &main_vpath,
        include_title,
    )?;

    fs::write(&output, index_html)
        .map_err(|e| format!("could not write output {}: {e}", output.display()))?;

    fs::write(&outline_file, outline_html.as_bytes())
        .map_err(|e| format!("could not write outline {}: {e}", outline_file.display()))?;

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
        outline.push_str(format!("<li><a href=\"#{}\">{}</a></li>\n", slug, header_text).as_str());

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
    info!("compiling all files at dir {:?}", root_dir);
    let res = compile_all_at(
        root_dir,
        root_dir,
        prepend,
        plugins,
        include_title_in_outline,
    );
    info!("done");
    return res;
}

fn compile_all_at(
    scan_dir: &PathBuf,
    root_dir: &PathBuf,
    prepend: &Option<PathBuf>,
    plugins: &[impl AsRef<str>],
    include_title_in_outline: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for entry in fs::read_dir(scan_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            compile_all_at(&path, root_dir, prepend, plugins, include_title_in_outline)?;
        } else if path.file_name().is_some_and(|n| n == "index.typ") {
            let dir = path.parent().unwrap().to_path_buf();
            if let Err(e) =
                compile_article(&dir, root_dir, prepend, plugins, include_title_in_outline)
            {
                error!("{e}");
            }
        }
    }

    Ok(())
}
