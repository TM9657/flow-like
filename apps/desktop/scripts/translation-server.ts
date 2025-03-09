import fs from "fs";
import path from "path";
import express from "express";

const app = express();
const PORT = 5544;

// Middleware to parse JSON bodies
app.use(express.json());

const pasePath = "public/locales";

// POST route handler
app.post("/", (req, res) => {
	const {
		lngs,
		namespace,
		key,
	}: { lngs: string[]; namespace: string; key: string } = req.body;
	const language = lngs[0];

	let data: any = {};
	const dir = path.join(pasePath, language, namespace + ".json");
	if (fs.existsSync(dir)) {
		data = JSON.parse(fs.readFileSync(dir, "utf-8"));
	}

	data[key] = key;

	fs.writeFileSync(dir, JSON.stringify(data, null, 2));
	res.status(200).json({ message: "Data received successfully" });
});

// Start server
app.listen(PORT, () => {
	console.log(`Server running on http://localhost:${PORT}`);
});
