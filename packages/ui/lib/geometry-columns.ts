export function isGeometryMetadata(
	metadata?: Record<string, unknown>,
): boolean {
	const extension = metadata?.["ARROW:extension:name"];
	return typeof extension === "string" && extension.startsWith("geoarrow.");
}

/** A map needs known WGS 84 coordinates; extension tags alone do not supply CRS. */
export function isWgs84GeometryMetadata(
	metadata?: Record<string, unknown>,
): boolean {
	if (!isGeometryMetadata(metadata)) return false;
	try {
		const text = metadata?.["ARROW:extension:metadata"];
		const extension = typeof text === "string" ? JSON.parse(text) : null;
		if (!extension || (extension.edges && extension.edges !== "planar"))
			return false;
		const crs = extension.crs;
		if (typeof crs === "string")
			return ["EPSG:4326", "OGC:CRS84", "urn:ogc:def:crs:OGC::CRS84"].includes(
				crs,
			);
		return crs?.id?.authority === "EPSG" && String(crs.id.code) === "4326";
	} catch {
		return false;
	}
}
