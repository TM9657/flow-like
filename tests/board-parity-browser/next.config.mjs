import { fileURLToPath } from "node:url";
import { existsSync, symlinkSync } from "node:fs";
const publicDir = new URL("./public", import.meta.url);
if (!existsSync(publicDir))
	symlinkSync("../../apps/web/public", publicDir, "dir");
export default {
	reactStrictMode: false,
	agentRules: false,
	images: { unoptimized: true },
	transpilePackages: ["@flow-like/flow-like-ui", "@flow-like/locales"],
	turbopack: { root: fileURLToPath(new URL("../../", import.meta.url)) },
};
