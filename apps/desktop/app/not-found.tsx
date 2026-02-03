"use client";

import { NotFoundPage } from "@tm9657/flow-like-ui";

export default function NotFound() {
	return (
		<NotFoundPage
			onGoBack={() => window.history.back()}
			homeHref="/"
		/>
	);
}
