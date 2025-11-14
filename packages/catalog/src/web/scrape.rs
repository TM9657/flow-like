use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod extract_links;

// use deno_core::{JsRuntime, RuntimeOptions};

// fn render_dynamic_content(html: &str) -> Result<String, Box<dyn std::error::Error>> {
//     let mut js_runtime = JsRuntime::new(RuntimeOptions::default());

//     let js_code = format!(
//         r#"
//         let document = {{ innerHTML: `{}` }};
//         let console = {{ log: function(msg) {{ Deno.core.print(msg + "\n"); }} }};
//         // Extract and execute scripts
//         let scripts = document.innerHTML.match(/<script[^>]*>([\s\S]*?)<\/script>/gi) || [];
//         for (let script of scripts) {{
//             let code = script.replace(/<script[^>]*>|<\/script>/gi, '');
//             eval(code);
//         }}
//         document.innerHTML; // Return modified content
//         "#,
//         html.replace("`", "\\`")
//     );

//     let result = js_runtime.execute_script("<anon>", js_code)?;
//     let mut scope = js_runtime.handle_scope();
//     let local = result.open(&mut scope);
//     let string_value = local.to_rust_string_lossy(&mut scope);

//     Ok(string_value)
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_basic_html_without_scripts() {
//         let html = "<div>Hello, world!</div>";
//         let result = render_dynamic_content(html).unwrap();
//         assert_eq!(result, "<div>Hello, world!</div>");
//     }

//     #[test]
//     fn test_html_with_script_modifying_inner_html() {
//         let html = r#"<div>Initial content</div><script>document.innerHTML = '<div>Modified content</div>';</script>"#;
//         let result = render_dynamic_content(html).unwrap();
//         assert_eq!(result, "<div>Modified content</div>");
//     }

//     #[test]
//     fn test_html_with_backticks() {
//         let html = "<div>`This has backticks`</div>";
//         let result = render_dynamic_content(html).unwrap();
//         assert_eq!(result, "<div>`This has backticks`</div>");
//     }

//     #[test]
//     fn test_empty_html() {
//         let html = "";
//         let result = render_dynamic_content(html).unwrap();
//         assert_eq!(result, "");
//     }

//     // #[test]
//     // fn test_multiple_scripts() {
//     //     let html = r#"
//     //         <div>Initial</div>
//     //         <script>document.innerHTML = document.innerHTML.replace('Initial', 'Step 1');</script>
//     //         <script>document.innerHTML = document.innerHTML.replace('Step 1', 'Final');</script>
//     //     "#;
//     //     let result = render_dynamic_content(html).unwrap();
//     //     println!("{}", result);
//     //     assert!(result.contains("Final"));
//     //     assert!(!result.contains("Initial"));
//     // }
// }
