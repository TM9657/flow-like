//! Macros for defining WASM nodes

/// Define a WASM node with declarative syntax
///
/// # Example
///
/// ```rust
/// node! {
///     name: "my_node",
///     friendly_name: "My Node",
///     description: "Does something useful",
///     category: "Custom/Example",
///
///     inputs: {
///         exec: Exec,
///         input_text: String = "default value",
///         count: I64 = 1,
///     },
///
///     outputs: {
///         exec_out: Exec,
///         result: String,
///     },
/// }
/// ```
#[macro_export]
macro_rules! node {
    (
        name: $name:literal,
        friendly_name: $friendly_name:literal,
        description: $description:literal,
        category: $category:literal,
        $(icon: $icon:literal,)?
        $(long_running: $long_running:literal,)?
        $(docs: $docs:literal,)?

        inputs: {
            $($input_name:ident : $input_type:ident $(= $input_default:expr)?),* $(,)?
        },

        outputs: {
            $($output_name:ident : $output_type:ident),* $(,)?
        } $(,)?
    ) => {
        #[no_mangle]
        pub extern "C" fn get_node() -> i64 {
            let mut def = $crate::NodeDefinition::new(
                $name,
                $friendly_name,
                $description,
                $category,
            );

            $(
                def.icon = Some($icon.to_string());
            )?

            $(
                def.long_running = Some($long_running);
            )?

            $(
                def.docs = Some($docs.to_string());
            )?

            // Add input pins
            $(
                let pin = $crate::PinDefinition::input(
                    stringify!($input_name),
                    &$crate::humanize_name(stringify!($input_name)),
                    concat!("Input: ", stringify!($input_name)),
                    $crate::type_to_data_type(stringify!($input_type)),
                );
                $(
                    let pin = pin.with_default(serde_json::json!($input_default));
                )?
                def.add_pin(pin);
            )*

            // Add output pins
            $(
                def.add_pin($crate::PinDefinition::output(
                    stringify!($output_name),
                    &$crate::humanize_name(stringify!($output_name)),
                    concat!("Output: ", stringify!($output_name)),
                    $crate::type_to_data_type(stringify!($output_type)),
                ));
            )*

            $crate::serialize_definition(&def)
        }
    };
}

/// Pin type aliases for the node! macro
pub mod pin_types {
    pub type Exec = ();
    pub type String = std::string::String;
    pub type I64 = i64;
    pub type F64 = f64;
    pub type Bool = bool;
    pub type Json = serde_json::Value;
    pub type Bytes = Vec<u8>;
}

/// Convert type name to data type string
pub fn type_to_data_type(type_name: &str) -> &'static str {
    match type_name {
        "Exec" => "Exec",
        "String" | "str" => "String",
        "I64" | "i64" | "I32" | "i32" | "Int" => "I64",
        "F64" | "f64" | "F32" | "f32" | "Float" => "F64",
        "Bool" | "bool" => "Bool",
        "Json" | "Object" | "Map" => "Generic",
        "Bytes" | "bytes" | "Binary" => "Bytes",
        "Array" | "Vec" | "List" => "Generic",
        _ => "Generic",
    }
}

/// Convert snake_case to Title Case
pub fn humanize_name(name: &str) -> std::string::String {
    name.split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => std::string::String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Serialize a node definition to WASM return format
pub fn serialize_definition(def: &crate::NodeDefinition) -> i64 {
    let json = serde_json::to_vec(def).unwrap_or_default();
    let len = json.len() as u32;

    // Allocate buffer and copy data
    let ptr = crate::alloc(len as i32) as u32;
    unsafe {
        std::ptr::copy_nonoverlapping(json.as_ptr(), ptr as *mut u8, len as usize);
    }

    // Pack pointer and length
    ((ptr as i64) << 32) | (len as i64)
}

/// Helper macro to implement the run function boilerplate
#[macro_export]
macro_rules! run_node {
    ($handler:expr) => {
        #[no_mangle]
        pub extern "C" fn run(ptr: i32, len: i32) -> i64 {
            let input_bytes = unsafe {
                std::slice::from_raw_parts(ptr as *const u8, len as usize)
            };

            let ctx = match $crate::Context::from_bytes(input_bytes) {
                Ok(ctx) => ctx,
                Err(e) => {
                    return $crate::ExecutionResult::error(e).to_wasm();
                }
            };

            let handler: fn($crate::Context) -> $crate::ExecutionResult = $handler;
            handler(ctx).to_wasm()
        }
    };
}

/// Define multiple nodes in a package
///
/// # Example
///
/// ```rust
/// package! {
///     nodes: [
///         {
///             name: "add",
///             friendly_name: "Add Numbers",
///             description: "Adds two numbers",
///             category: "Math/Arithmetic",
///             inputs: { exec: Exec, a: I64 = 0, b: I64 = 0 },
///             outputs: { exec_out: Exec, result: I64 },
///         },
///         {
///             name: "subtract",
///             friendly_name: "Subtract Numbers",
///             description: "Subtracts two numbers",
///             category: "Math/Arithmetic",
///             inputs: { exec: Exec, a: I64 = 0, b: I64 = 0 },
///             outputs: { exec_out: Exec, result: I64 },
///         }
///     ]
/// }
/// ```
#[macro_export]
macro_rules! package {
    (
        nodes: [
            $(
                {
                    name: $name:literal,
                    friendly_name: $friendly_name:literal,
                    description: $description:literal,
                    category: $category:literal,
                    $(icon: $icon:literal,)?
                    $(long_running: $long_running:literal,)?
                    $(docs: $docs:literal,)?

                    inputs: {
                        $($input_name:ident : $input_type:ident $(= $input_default:expr)?),* $(,)?
                    },

                    outputs: {
                        $($output_name:ident : $output_type:ident),* $(,)?
                    } $(,)?
                }
            ),+ $(,)?
        ]
    ) => {
        #[no_mangle]
        pub extern "C" fn get_nodes() -> i64 {
            let mut nodes = $crate::PackageNodes::new();

            $(
                {
                    let mut def = $crate::NodeDefinition::new(
                        $name,
                        $friendly_name,
                        $description,
                        $category,
                    );

                    $(
                        def.icon = Some($icon.to_string());
                    )?

                    $(
                        def.long_running = Some($long_running);
                    )?

                    $(
                        def.docs = Some($docs.to_string());
                    )?

                    // Add input pins
                    $(
                        let pin = $crate::PinDefinition::input(
                            stringify!($input_name),
                            &$crate::humanize_name(stringify!($input_name)),
                            concat!("Input: ", stringify!($input_name)),
                            $crate::type_to_data_type(stringify!($input_type)),
                        );
                        $(
                            let pin = pin.with_default(serde_json::json!($input_default));
                        )?
                        def.add_pin(pin);
                    )*

                    // Add output pins
                    $(
                        def.add_pin($crate::PinDefinition::output(
                            stringify!($output_name),
                            &$crate::humanize_name(stringify!($output_name)),
                            concat!("Output: ", stringify!($output_name)),
                            $crate::type_to_data_type(stringify!($output_type)),
                        ));
                    )*

                    nodes.add_node(def);
                }
            )+

            nodes.to_wasm()
        }
    };
}

/// Serialize package nodes to WASM return format
pub fn serialize_package_nodes(nodes: &crate::PackageNodes) -> i64 {
    let json = serde_json::to_vec(nodes).unwrap_or_default();
    let len = json.len() as u32;

    let ptr = crate::alloc(len as i32) as u32;
    unsafe {
        std::ptr::copy_nonoverlapping(json.as_ptr(), ptr as *mut u8, len as usize);
    }

    ((ptr as i64) << 32) | (len as i64)
}

/// Helper macro to implement the run function for multi-node packages.
///
/// For packages with multiple nodes, this macro creates a dispatcher
/// that routes execution to the correct node handler based on node name.
///
/// # Example
///
/// ```rust
/// run_package!(handle_run);
///
/// fn handle_run(node_name: &str, ctx: Context) -> ExecutionResult {
///     match node_name {
///         "add" => { /* handle add */ ctx.success() }
///         "subtract" => { /* handle subtract */ ctx.success() }
///         _ => ctx.fail(format!("Unknown node: {}", node_name))
///     }
/// }
/// ```
#[macro_export]
macro_rules! run_package {
    ($handler:expr) => {
        #[no_mangle]
        pub extern "C" fn run(ptr: i32, len: i32) -> i64 {
            let input_bytes = unsafe {
                std::slice::from_raw_parts(ptr as *const u8, len as usize)
            };

            let ctx = match $crate::Context::from_bytes(input_bytes) {
                Ok(ctx) => ctx,
                Err(e) => {
                    return $crate::ExecutionResult::error(e).to_wasm();
                }
            };

            // Extract node name from context
            let node_name = ctx.node_name().to_string();

            let handler: fn(&str, $crate::Context) -> $crate::ExecutionResult = $handler;
            handler(&node_name, ctx).to_wasm()
        }
    };
}
