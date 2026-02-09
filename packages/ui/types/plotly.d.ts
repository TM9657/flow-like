declare module "plotly.js-dist-min" {
	const Plotly: {
		react: (
			root: HTMLElement,
			data: unknown[],
			layout?: Record<string, unknown>,
			config?: Record<string, unknown>,
		) => Promise<void>;
		purge: (root: HTMLElement) => void;
		newPlot: (
			root: HTMLElement,
			data: unknown[],
			layout?: Record<string, unknown>,
			config?: Record<string, unknown>,
		) => Promise<void>;
		relayout: (
			root: HTMLElement,
			layout: Record<string, unknown>,
		) => Promise<void>;
		restyle: (
			root: HTMLElement,
			update: Record<string, unknown>,
			indices?: number | number[],
		) => Promise<void>;
	};
	export default Plotly;
}
