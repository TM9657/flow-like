use super::*;
use serde_json::json;

const HUB: &str = "models.example";

fn model(id: &str, assets: Value) -> Value {
    json!({
        "id": id,
        "ref": id,
        "type": "Forecasting",
        "file_name": "model.safetensors",
        "parameters": {
            "model_type": "chronos2",
            "provider": {"provider_name": "local:forecast"},
            "assets": assets
        }
    })
}

fn config() -> Value {
    json!({"id": "config-plan", "ref": "config", "type": "Config", "file_name": "config.json"})
}

fn stored(id: &str, hub: &str) -> Bit {
    Bit {
        id: id.into(),
        hub: hub.into(),
        ..Bit::default()
    }
}

#[test]
fn asset_aliases_order_publication_and_use_returned_ids_in_both_locations() {
    let mut forecast = model(
        "model-plan",
        json!([
            {"bit": "@config", "relative_path": "config.json"},
            {"bit": "@weights", "relative_path": "model-00001-of-00002.safetensors"},
            {"bit": "external.example:remote-shard", "relative_path": "model-00002-of-00002.safetensors"}
        ]),
    );
    forecast["dependencies"] = json!(["@config"]);
    let planned = plan(
        &Spec {
            bits: vec![
                forecast,
                config(),
                json!({"id": "weights-plan", "ref": "weights", "type": "File"}),
            ],
        },
        HUB,
    )
    .unwrap();
    assert_eq!(
        planned
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["config-plan", "weights-plan", "model-plan"]
    );
    let root = planned.last().unwrap();
    assert_eq!(
        root.dependencies.len(),
        3,
        "explicit and asset dependencies deduplicate"
    );
    let returned = BTreeMap::from([
        (
            "config-plan".into(),
            stored("existing-config", "canonical.example"),
        ),
        ("weights-plan".into(), stored("new-weights", "")),
    ]);
    let payload = root.push_payload(&returned, HUB).unwrap();
    assert_eq!(
        payload["dependencies"],
        json!([
            "canonical.example:existing-config",
            "models.example:new-weights",
            "external.example:remote-shard"
        ])
    );
    let assets = payload["parameters"]["assets"].as_array().unwrap();
    assert_eq!(assets[0]["bit"], "canonical.example:existing-config");
    assert_eq!(assets[1]["bit"], "models.example:new-weights");
    assert_eq!(assets[2]["bit"], "external.example:remote-shard");
    assert_eq!(assets[0]["relative_path"], "config.json");
    assert_eq!(payload["parameters"]["model_type"], "chronos2");
    assert_eq!(root.payload["parameters"]["assets"][0]["bit"], "@config");
    assert!(root.push_payload(&BTreeMap::new(), HUB).is_err());
}

#[test]
fn remote_asset_references_preserve_their_hub_and_qualify_bare_ids() {
    let planned = plan(
        &Spec {
            bits: vec![model(
                "model",
                json!([
                    {"bit": "existing-config", "relative_path": "config.json"},
                    {"bit": "other.example:weights", "relative_path": "weights.safetensors"}
                ]),
            )],
        },
        HUB,
    )
    .unwrap();
    let payload = planned[0].push_payload(&BTreeMap::new(), HUB).unwrap();
    assert_eq!(
        payload["parameters"]["assets"][0]["bit"],
        "models.example:existing-config"
    );
    assert_eq!(
        payload["parameters"]["assets"][1]["bit"],
        "other.example:weights"
    );
    assert_eq!(
        payload["dependencies"],
        json!(["models.example:existing-config", "other.example:weights"])
    );
}

#[test]
fn unknown_asset_aliases_and_asset_dependency_cycles_fail_before_publication() {
    let undefined = plan(
        &Spec {
            bits: vec![model(
                "model",
                json!([{"bit": "@missing", "relative_path": "config.json"}]),
            )],
        },
        HUB,
    );
    assert!(undefined.err().unwrap().to_string().contains("@missing"));
    let cyclic = plan(
        &Spec {
            bits: vec![
                model(
                    "first",
                    json!([{"bit": "@second", "relative_path": "config.json"}]),
                ),
                model(
                    "second",
                    json!([{"bit": "@first", "relative_path": "config.json"}]),
                ),
            ],
        },
        HUB,
    );
    assert!(
        cyclic
            .err()
            .unwrap()
            .to_string()
            .contains("dependency cycle")
    );
}

#[test]
fn root_file_mappings_do_not_create_self_dependencies_or_predict_new_root_ids() {
    for root_reference in [
        "@root",
        "root",
        "models.example:root",
        "https://models.example:root",
    ] {
        let planned = plan(
            &Spec {
                bits: vec![
                    model(
                        "root",
                        json!([
                            {"bit": "@config", "relative_path": "config.json"},
                            {"bit": root_reference, "relative_path": "model.safetensors"}
                        ]),
                    ),
                    config(),
                ],
            },
            HUB,
        )
        .unwrap();
        let root = planned.last().unwrap();
        assert_eq!(
            root.dependencies,
            vec![Dependency::Local("config-plan".into())]
        );
        assert_eq!(
            root.payload["parameters"]["assets"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(root.payload["file_name"], "model.safetensors");
    }
    let renamed = plan(
        &Spec {
            bits: vec![
                model(
                    "root",
                    json!([
                        {"bit": "@config", "relative_path": "config.json"},
                        {"bit": "@root", "relative_path": "renamed.safetensors"}
                    ]),
                ),
                config(),
            ],
        },
        HUB,
    );
    assert!(
        renamed
            .err()
            .unwrap()
            .to_string()
            .contains("root file_name")
    );
}

#[test]
fn pulled_asset_aliases_republish_without_reusing_old_dependency_identities() {
    let config = Bit {
        bit_type: BitTypes::Config,
        ..stored("old-config", HUB)
    };
    let root = Bit {
        bit_type: BitTypes::Forecasting,
        file_name: Some("model.safetensors".into()),
        parameters: json!({
            "model_type": "timestral",
            "assets": [
                {"bit": "models.example:old-config", "relative_path": "config.json"},
                {"bit": "foreign.example:old-config", "relative_path": "foreign.safetensors"},
                {"bit": "old-model", "relative_path": "model.safetensors"}
            ]
        }),
        ..stored("old-model", HUB)
    };
    let exported = from_bits(&[config, root]);
    assert_eq!(
        exported.bits[1]["parameters"]["assets"][0]["bit"],
        "@config-0"
    );
    assert_eq!(
        exported.bits[1]["parameters"]["assets"][1]["bit"],
        "foreign.example:old-config"
    );
    assert_eq!(
        exported.bits[1]["parameters"]["assets"][2]["bit"],
        "@forecasting-1"
    );
    let planned = plan(&exported, HUB).unwrap();
    let payload = planned[1]
        .push_payload(
            &BTreeMap::from([("old-config".into(), stored("returned-config", HUB))]),
            HUB,
        )
        .unwrap();
    assert_eq!(
        payload["parameters"]["assets"][0]["bit"],
        "models.example:returned-config"
    );
    assert_eq!(payload["parameters"]["assets"].as_array().unwrap().len(), 2);
}

#[test]
fn forecasting_validation_runs_before_any_hub_write() {
    for assets in [
        json!([{"bit": "config", "relative_path": "../config.json"}]),
        json!([{"bit": "config", "relative_path": "weights.safetensors"}]),
        json!([{"bit": "config", "relative_path": "config.json"}, {"bit": "another", "relative_path": "config.json"}]),
    ] {
        assert!(
            plan(
                &Spec {
                    bits: vec![model("model", assets)]
                },
                HUB
            )
            .is_err()
        );
    }
}

#[test]
fn release_catalog_pins_sources_and_resolves_every_checkpoint_and_license_asset() {
    let bundle =
        load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("specs/forecasting.json"))
            .unwrap();
    let planned = plan(&bundle, HUB).unwrap();
    assert_eq!(planned.len(), 15);
    let mut returned = BTreeMap::new();
    let mut families = BTreeMap::<String, usize>::new();
    for (index, entry) in planned.iter().enumerate() {
        let payload = entry.push_payload(&returned, HUB).unwrap();
        let source = payload["download_link"].as_str().unwrap();
        let pieces = source.split('/').collect::<Vec<_>>();
        let revision = match pieces[2] {
            "huggingface.co" => {
                let resolve = pieces.iter().position(|part| *part == "resolve").unwrap();
                let revision = pieces[resolve + 1];
                assert_eq!(payload["version"], revision);
                revision
            }
            "raw.githubusercontent.com" => pieces[5],
            other => panic!("Unexpected release download host: {other}"),
        };
        assert_eq!(revision.len(), 40, "release URLs must pin complete commits");
        assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        if entry.bit_type == "Forecasting" {
            let parameters: ForecastingModelParameters =
                serde_json::from_value(payload["parameters"].clone()).unwrap();
            parameters.validate().unwrap();
            let family = payload["parameters"]["model_type"].as_str().unwrap();
            *families.entry(family.into()).or_default() += 1;
            assert_eq!(payload["parameters"]["provider"]["version"], revision);
            for name in ["config.json", "LICENSE"] {
                let asset = parameters
                    .assets
                    .iter()
                    .find(|asset| asset.relative_path == name)
                    .unwrap();
                assert!(returned.values().any(|bit: &Bit| {
                    asset.bit == format!("{}:{}", bit.hub, bit.id)
                        && bit.file_name.as_deref() == Some(name)
                }));
                assert!(
                    payload["dependencies"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(asset.bit))
                );
            }
            if family == "timestral" {
                assert_eq!(payload["license"], "Modified MIT (revenue restriction)");
                let metadata = serde_json::to_value(entry.meta.get("en").unwrap()).unwrap();
                let terms = metadata["long_description"].as_str().unwrap();
                assert!(terms.contains("US$20 million") && terms.contains("preceding month"));
                assert!(
                    metadata["tags"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("restricted-license"))
                );
                let license = parameters
                    .assets
                    .iter()
                    .find(|asset| asset.relative_path == "LICENSE")
                    .unwrap();
                let license_bit = returned
                    .values()
                    .find(|bit| license.bit == format!("{}:{}", bit.hub, bit.id))
                    .unwrap();
                assert_eq!(
                    license_bit.license,
                    Some("Modified MIT (revenue restriction)".into())
                );
            }
        }
        let mut saved: Bit = serde_json::from_value(payload).unwrap();
        saved.id = format!("actual-hub-id-{index}");
        saved.hub = "canonical.example".into();
        returned.insert(entry.id.clone(), saved);
    }
    assert_eq!(
        families,
        BTreeMap::from([
            ("timestral".into(), 2),
            ("chronos2".into(), 1),
            ("toto2".into(), 1),
            ("timesfm2_5".into(), 1),
            ("tiny_time_mixer".into(), 1),
        ])
    );
}
