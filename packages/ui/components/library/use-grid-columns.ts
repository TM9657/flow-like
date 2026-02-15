"use client";

import { useEffect, useState } from "react";

export function useGridColumns(
	containerRef: React.RefObject<HTMLDivElement | null>,
	cardMin: number,
) {
	const [cols, setCols] = useState(5);

	useEffect(() => {
		const el = containerRef.current;
		if (!el) return;

		const observer = new ResizeObserver(([entry]) => {
			if (!entry) return;
			const width = entry.contentRect.width;
			setCols(Math.max(1, Math.floor((width + 16) / (cardMin + 16))));
		});

		observer.observe(el);
		return () => observer.disconnect();
	}, [containerRef, cardMin]);

	return cols;
}
