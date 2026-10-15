import "@xyflow/react/dist/style.css";
import "../../../packages/ui/global.css";
import "./style.css";
export default function Layout({ children }: { children: React.ReactNode }) {
	return (
		<html lang="en" suppressHydrationWarning>
			<body>{children}</body>
		</html>
	);
}
