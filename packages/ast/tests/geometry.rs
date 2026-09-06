use flow_like_ast::{Container, InterfaceType, RenderOptions, TypeRef, parse, render};
use flow_like_types_contracts::geometry::{GeometryKind, marker};

#[test]
fn geometry_subtypes_round_trip_through_variables_containers_and_functions() {
    let source = "const origin: geometry<Point> = {\"type\":\"Point\",\"coordinates\":[13.405,52.52]}\nconst shapes: geometry[]\nconst boundaries: Map<string, geometry<Polygon>>\nconst locations: Set<geometry<Point>>\n\nfunction locate(position: geometry<Point>): (shape: geometry) {\n    return position\n}\n";
    let ast = parse(source).expect("geometry declarations parse");
    assert_eq!(ast.variables[0].ty.geometry_kind, Some(GeometryKind::Point));
    assert_eq!(
        ast.variables[0].schema.as_deref(),
        Some(marker(GeometryKind::Point))
    );
    assert_eq!(ast.variables[2].ty.container, Container::Map);
    assert_eq!(
        ast.variables[2].ty.geometry_kind,
        Some(GeometryKind::Polygon)
    );
    assert_eq!(
        ast.functions[0].params[0].ty.geometry_kind,
        Some(GeometryKind::Point)
    );
    let text = render(&ast, &RenderOptions::default());
    assert!(
        !text.contains("@schema"),
        "subtypes are readable type annotations: {text}"
    );
    let reparsed = parse(&text).expect("rendered geometry parses");
    assert_eq!(reparsed.variables[0].schema, ast.variables[0].schema);
    assert_eq!(render(&reparsed, &RenderOptions::default()), text);
}

#[test]
fn older_ast_type_refs_default_to_no_geometry_subtype() {
    let ty: TypeRef = serde_json::from_str(r#"{"base":"string","container":"Normal"}"#).unwrap();
    assert_eq!(ty.geometry_kind, None);
}

#[test]
fn geometry_defaults_validate_subtypes_and_containers() {
    for source in [
        "const shape: geometry<Point> = {\"type\":\"Point\",\"coordinates\":[13,52]}",
        "const shapes: geometry<Point>[] = [{\"type\":\"Point\",\"coordinates\":[13,52]}]",
        "const shapes: Set<geometry<Point>> = [{\"type\":\"Point\",\"coordinates\":[13,52]}]",
        "const shapes: Map<string, geometry<Point>> = {\"berlin\":{\"type\":\"Point\",\"coordinates\":[13,52]}}",
    ] {
        parse(source).expect("valid geometry defaults parse");
    }
    for source in [
        "const shape: geometry = null",
        "const shape: geometry = \"Point\"",
        "const shape: geometry<Point> = {\"type\":\"Point\",\"coordinates\":[181,52]}",
        "const shape: geometry<Polygon> = {\"type\":\"Point\",\"coordinates\":[13,52]}",
        "const shapes: geometry<Point>[] = [{\"type\":\"Point\",\"coordinates\":[13,52]},null]",
        "const shapes: Map<string, geometry<Point>> = []",
    ] {
        assert!(parse(source).is_err(), "invalid default accepted: {source}");
    }
}

#[test]
fn geometry_markers_do_not_generate_interfaces_and_conflicting_annotations_fail() {
    let schema = flow_like_ast::quote_string(marker(GeometryKind::Point));
    let ast = parse(&format!("@schema({schema})\nconst origin: geometry\n")).unwrap();
    assert_eq!(ast.variables[0].ty.geometry_kind, Some(GeometryKind::Point));
    assert!(flow_like_ast::interfaces_for_variables(&ast.variables).is_empty());
    assert!(
        parse(&format!(
            "@schema({schema})\nconst shape: geometry<Polygon>\n"
        ))
        .is_err()
    );
    assert!(parse(&format!("@schema({schema})\nconst shape: Struct\n")).is_err());
    assert!(parse("const shape: geometry<Triangle>\n").is_err());
    assert!(parse("const shape: Map<int, geometry<Point>>\n").is_err());
}

#[test]
fn geometry_interface_fields_expand_and_round_trip_without_dangling_definitions() {
    let source = "interface Place {\n    location: geometry<Point>;\n    shapes: geometry[];\n    grouped: Map<string, geometry<GeometryCollection>>;\n}\n\nconst place: Place\n";
    let ast = parse(source).expect("geometry fields parse");
    let schema: serde_json::Value =
        serde_json::from_str(ast.variables[0].schema.as_deref().unwrap()).unwrap();
    assert_eq!(schema["properties"]["location"]["x-geometry"], "Point");
    fn check_refs<'a>(value: &'a serde_json::Value, root: &'a serde_json::Value) {
        let root = if value.get("$id").is_some() {
            value
        } else {
            root
        };
        if let Some(reference) = value.get("$ref").and_then(serde_json::Value::as_str) {
            if let Some(pointer) = reference.strip_prefix('#') {
                assert!(
                    root.pointer(pointer).is_some(),
                    "unresolved geometry schema ref {reference}"
                );
            }
        }
        match value {
            serde_json::Value::Object(object) => {
                for child in object.values() {
                    check_refs(child, root);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    check_refs(child, root);
                }
            }
            _ => {}
        }
    }
    check_refs(&schema, &schema);
    let interfaces = flow_like_ast::interfaces_for_variables(&ast.variables);
    assert!(matches!(
        interfaces[0].fields[0].ty,
        InterfaceType::Geometry(_) | InterfaceType::Map(_) | InterfaceType::Array(_)
    ));
    let text = render(&ast, &RenderOptions::default());
    assert!(text.contains("geometry<Point>"));
    assert_eq!(
        render(&parse(&text).unwrap(), &RenderOptions::default()),
        text
    );
}
