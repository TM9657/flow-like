import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepareBrowserCodecs({
	directory,
	original,
	write,
	replaceOnce,
}) {
	const local = (name) => readFileSync(join(directory, name), "utf8");
	const between = (source, start, end, replacement, label) => {
		const first = source.indexOf(start);
		const last = source.indexOf(end, first + start.length);
		if (
			first < 0 ||
			last < 0 ||
			source.indexOf(start, first + start.length) >= 0
		)
			throw new Error(`Film browser codec patch needs review: ${label}`);
		return source.slice(0, first) + replacement + source.slice(last);
	};
	write("apps/filmcraft-web/src/browser-codecs.js", local("browser-codecs.js"));
	write("apps/filmcraft-web/src/webcodecs.rs", local("browser-media.rs"));
	write("apps/filmcraft-web/src/browser_export.rs", local("browser-export.rs"));
	let webCargo = original("apps/filmcraft-web/Cargo.toml");
	webCargo = replaceOnce(
		webCargo,
		"filmcraft-engine = { workspace = true }",
		"filmcraft-engine = { workspace = true }\nfilmcraft-project = { workspace = true }\nfilmcraft-render = { workspace = true }\nfilmcraft-matroska = { workspace = true }",
		"browser export compositor dependencies",
	);
	write("apps/filmcraft-web/Cargo.toml", webCargo);
	let mux = original("crates/isobmff/src/mux.rs");
	mux = replaceOnce(
		mux,
		'    let tb = &t.t;\n    let m = b.start_full(b"stts", 0, 0);',
		`    let tb = &t.t;
    // Apple readers need roll groups to interpret AAC edits as explicit priming.
    // https://developer.apple.com/documentation/quicktime-file-format/using_track_structures_to_represent_encode_delay_explictly
    if matches!(&t.cfg.entry.codec, CodecConfig::Aac(config) if config.object_type == 2)
        && !edits.is_empty()
        && tb.sample_count > 0
    {
        let sample_count = u32::try_from(tb.sample_count)
            .map_err(|_| Error::Mux("AAC sample count exceeds roll group capacity".into()))?;
        let group = b.start_full(b"sgpd", 1, 0);
        b.bytes(b"roll");
        b.u32(2);
        b.u32(1);
        b.i16(-1);
        b.end(group);
        let mapping = b.start_full(b"sbgp", 0, 0);
        b.bytes(b"roll");
        b.u32(1);
        b.u32(sample_count);
        b.u32(1);
        b.end(mapping);
    }
    let m = b.start_full(b"stts", 0, 0);`,
		"explicit AAC priming sample groups",
	);
	write("crates/isobmff/src/mux.rs", mux);
	// The replacement registry has no decoder implementation dependencies.
	write("crates/codecs/src/lib.rs", local("browser-registry.rs"));
	let codecsCargo = original("crates/codecs/Cargo.toml");
	codecsCargo = between(
		codecsCargo,
		"[dependencies]\n",
		"[dev-dependencies]\n",
		"[dependencies]\nfilmcraft-media = { workspace = true }\n\n",
		"codec dependency isolation",
	);
	write("crates/codecs/Cargo.toml", codecsCargo);
	let perf = original("crates/engine/src/perf.rs");
	perf = between(
		perf,
		"pub fn decode_json() -> Value {",
		"/// Export counters:",
		`pub fn decode_json() -> Value {
    json!({"backend": "browser", "softwareFallback": false})
}

`,
		"browser decoder diagnostics",
	);
	write("crates/engine/src/perf.rs", perf);
	let exportCargo = original("crates/export/Cargo.toml");
	for (const name of ["prores", "dnx", "apv", "aac", "h264enc"]) {
		exportCargo = replaceOnce(
			exportCargo,
			`filmcraft-${name} = { workspace = true }\n`,
			"",
			`remove ${name} encoder dependency`,
		);
	}
	write("crates/export/Cargo.toml", exportCargo);
	let exp = original("crates/export/src/lib.rs");
	exp = between(
		exp,
		"/// AAC-LC (our encoder),",
		"/// FLAC (our encoder)",
		"",
		"remove AAC implementation",
	);
	exp = between(
		exp,
		"fn aac_factory(",
		"/// The range to export",
		"",
		"remove software video encoder implementations",
	);
	exp = exp.replaceAll("filmcraft_h264enc::PassStats", "()");
	exp = replaceOnce(
		exp,
		"vec![h264_factory, prores_factory, dnx_factory, apv_factory, mjpeg_factory]",
		"vec![mjpeg_factory]",
		"native encoder registry",
	);
	exp = replaceOnce(
		exp,
		"vec![aac_factory]",
		"Vec::new()",
		"native audio encoder registry",
	);
	exp = replaceOnce(
		exp,
		"!matches!(self, Format::Hevc | Format::Av1)",
		"!matches!(self, Format::H264 | Format::Hevc | Format::Av1 | Format::Aac | Format::ProRes | Format::DnxHr | Format::Apv | Format::MxfOp1a | Format::MxfOpAtom)",
		"unsupported export formats",
	);
	write("crates/export/src/lib.rs", exp);
	let settings = original("crates/export/src/settings.rs");
	settings = replaceOnce(
		settings,
		"crate::prores_profile(&self.prores_profile).chroma() == filmcraft_prores::ChromaFormat::Yuv444",
		"false",
		"ProRes alpha availability",
	);
	settings = replaceOnce(
		settings,
		"crate::prores_profile(&self.prores_profile).nominal_mbps_1080p30()",
		"220.0",
		"unused ProRes estimate",
	);
	settings = between(
		settings,
		"let bpp = match crate::apv_profile(&self.apv_profile) {",
		"                px * fps * bpp",
		"let bpp = 3.0;\n",
		"unused APV estimate",
	);
	settings = between(
		settings,
		"                    use filmcraft_prores::Profile;",
		"                Format::DnxHr =>",
		'                    v += ", unavailable in this browser build";\n                }\n',
		"ProRes export summary",
	);
	settings = replaceOnce(
		settings,
		'Format::Apv => v += &format!(", {}", crate::apv_profile(&self.apv_profile).name()),',
		'Format::Apv => v += ", unavailable in this browser build",',
		"APV export summary",
	);
	write("crates/export/src/settings.rs", settings);
	let mxf = original("crates/export/src/mxf_out.rs");
	mxf = replaceOnce(
		mxf,
		"Format, H264Profile, Out",
		"Format, Out",
		"remove unused encoder profile import",
	);
	mxf = between(
		mxf,
		"    let coding = match settings.video_format() {",
		"    let mut d = PictureDesc::new(coding, w, h);",
		"    let coding = PictureCoding::Avc { profile_idc: 100, intra: false };\n",
		"remove unused MXF codec metadata dependencies",
	);
	write("crates/export/src/mxf_out.rs", mxf);
	let manager = original("crates/engine/src/project_manager.rs");
	manager = replaceOnce(
		manager,
		"filmcraft_export::prores_profile(pr.prores).nominal_mbps_1080p30()",
		"220.0",
		"unused proxy estimate",
	);
	write("crates/engine/src/project_manager.rs", manager);
}
