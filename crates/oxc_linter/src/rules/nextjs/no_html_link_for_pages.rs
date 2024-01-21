use std::path::Path;

use glob::glob;
use oxc_ast::{
    ast::{JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElementName, JSXIdentifier},
    AstKind,
};
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::{self, Error},
};
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

#[derive(Debug, Error, Diagnostic)]
#[error("eslint-plugin-next(no-html-link-for-pages):")]
#[diagnostic(severity(warning), help(""))]
struct NoHtmlLinkForPagesDiagnostic(#[label] pub Span);

#[derive(Debug, Default, Clone)]
pub struct NoHtmlLinkForPages(Box<NoHtmlLinkForPagesConfig>);

#[derive(Debug, Default, Clone)]
pub struct NoHtmlLinkForPagesConfig {
    pages_dir: Vec<String>,
}

impl std::ops::Deref for NoHtmlLinkForPages {
    type Target = NoHtmlLinkForPagesConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

declare_oxc_lint!(
    /// ### What it does
    ///
    ///
    /// ### Why is this bad?
    ///
    ///
    /// ### Example
    /// ```javascript
    /// ```
    NoHtmlLinkForPages,
    correctness
);

impl Rule for NoHtmlLinkForPages {
    fn from_configuration(value: serde_json::Value) -> Self {
        let options: Option<&serde_json::Value> = value.get(0);
        let pages_dir = match options {
            Some(serde_json::Value::Array(val)) => {
                val.iter().map(|v| v.as_str().unwrap().to_string()).collect()
            }
            Some(serde_json::Value::String(val)) => vec![val.to_string()],
            _ => vec![],
        };

        Self(Box::new(NoHtmlLinkForPagesConfig { pages_dir }))
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let root_dir = &ctx.settings().nextjs.root_dir;
        let root_dirs = get_root_dirs(root_dir);
        dbg!(&root_dirs);

        let pages_dirs: Vec<String> = if self.pages_dir.is_empty() {
            root_dirs
                .iter()
                .flat_map(|dir| vec![format!("{}/pages", dir), format!("{}/src/pages", dir)])
                .collect()
        } else {
            self.pages_dir.clone()
        };
        dbg!(&pages_dirs);

        let app_dirs: Vec<String> = root_dirs
            .iter()
            .flat_map(|dir| vec![format!("{}/app", dir), format!("{}/src/app", dir)])
            .collect();

        dbg!(&app_dirs);

        // todo: https://github.com/vercel/next.js/pull/51783

        match node.kind() {
            AstKind::JSXOpeningElement(jsx_el) => {
                let JSXElementName::Identifier(JSXIdentifier { name: tag_name, .. }) = &jsx_el.name
                else {
                    return;
                };
                if tag_name.as_str() != "a" {
                    return;
                }
                dbg!(&jsx_el);

                if jsx_el.attributes.len() == 0 {
                    return;
                }

                let target = jsx_el.attributes.iter().find(|attr| {
                    matches!(
                        attr,
                        JSXAttributeItem::Attribute(jsx_attr)
                            if matches!(
                                &jsx_attr.name,
                                JSXAttributeName::Identifier(id) if id.name.as_str() == "target"
                            )
                    )
                });
                if let Some(JSXAttributeItem::Attribute(target)) = target {
                    if let Some(JSXAttributeValue::StringLiteral(target_value)) = &target.value {
                        if target_value.value == "_blank" {
                            println!("target is _blank");
                            return;
                        }
                    }
                }

                let Some(href) = jsx_el.attributes.iter().find(|attr| {
                    matches!(
                        attr,
                        JSXAttributeItem::Attribute(jsx_attr)
                            if matches!(
                                &jsx_attr.name,
                                JSXAttributeName::Identifier(id) if id.name.as_str() == "href"
                            )
                    )
                }) else {
                    return;
                };

                let has_download_attr = jsx_el.attributes.iter().any(|attr| {
                    matches!(attr, JSXAttributeItem::Attribute(jsx_attr) if matches!(&jsx_attr.name, JSXAttributeName::Identifier(id) if id.name == "download"))
                });

                if has_download_attr {
                    return;
                }

                let href_path = if let JSXAttributeItem::Attribute(href) = href {
                    if let Some(JSXAttributeValue::StringLiteral(value)) = &href.value {
                        if let Some(path) = normalize_url(value.value.to_string()) {
                            path
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    return;
                };

                if href_path.starts_with("http://")
                    || href_path.starts_with("https://")
                    || href_path.starts_with("//")
                {
                    return;
                }

                dbg!(ctx.settings());
            }
            _ => {}
        }
    }
}

fn normalize_url(mut url: String) -> Option<String> {
    if url.is_empty() {
        return None;
    }

    if let Some(index) = url.find('?') {
        url.truncate(index);
    }

    if let Some(index) = url.find('#') {
        url.truncate(index);
    }

    url = url.replace("/index.html", "/");

    if url.is_empty() {
        return Some(url);
    }

    if !url.ends_with('/') {
        url.push('/');
    }

    Some(url)
}

// Gets one or more Root, returns an array of root directories.
fn get_root_dirs(root_dir: &Vec<String>) -> Vec<String> {
    let processed_dirs: Vec<String> =
        root_dir.iter().flat_map(|dir| process_root_dir(dir)).collect();

    processed_dirs
}

// Process a Next.js root directory glob.
fn process_root_dir(root_dir: &str) -> Vec<String> {
    // Ensure we only match folders.
    let root_dir =
        if !root_dir.ends_with('/') { format!("{}/", root_dir) } else { root_dir.to_string() };

    glob_sync(&root_dir)
}

fn glob_sync(pattern: &str) -> Vec<String> {
    let mut result = Vec::new();

    if let Ok(entries) = glob(pattern) {
        for entry in entries {
            if let Ok(path) = entry {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    result.push(file_name.to_string());
                }
            }
        }
    }

    result
}

#[test]
fn test() {
    use crate::tester::Tester;
    use std::path::PathBuf;

    let pass = vec![(
        r#"import Link from 'next/link';

            export class Blah extends Head {
              render() {
                return (
                  <div>
                    <Link href='/'>
                      <a>Homepage</a>
                    </Link>
                    <h1>Hello title</h1>
                  </div>
                );
              }
            }
			"#,
        None,
        None,
        Some(PathBuf::from("pages/_document.js")),
    )];

    let fail = vec![];

    Tester::new(NoHtmlLinkForPages::NAME, pass, fail).test_and_snapshot();
}
