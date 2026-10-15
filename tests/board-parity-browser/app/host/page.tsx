"use client";

import dynamic from "next/dynamic";

const HostFixture = dynamic(() => import("../../host-fixture"), { ssr: false });

export default function Page() {
	return <HostFixture />;
}
