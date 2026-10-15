"use client";
import dynamic from "next/dynamic";
const Fixture = dynamic(() => import("../fixture"), { ssr: false });
export default function Page() {
	return <Fixture />;
}
