import {
	Breadcrumb,
	BreadcrumbItem,
	BreadcrumbLink,
	BreadcrumbList,
	BreadcrumbPage,
	BreadcrumbSeparator,
} from "../ui";

export function StorageBreadcrumbs({
	appId,
	prefix,
	updatePrefix,
}: Readonly<{
	appId: string;
	prefix: string;
	updatePrefix: (prefix: string) => void;
}>) {
	const segments = prefix.split("/").filter((segment) => segment !== "");

	return (
		<Breadcrumb>
			<BreadcrumbList>
				<BreadcrumbItem>
					<BreadcrumbLink
						className="cursor-pointer"
						onClick={(e) => {
							e.preventDefault();
							updatePrefix("");
						}}
					>
						Uploads
					</BreadcrumbLink>
				</BreadcrumbItem>
				{segments.slice(0, -1).map((part, index) => (
					<>
						<BreadcrumbSeparator />
						<BreadcrumbItem>
							<BreadcrumbLink
								key={index + part}
								className="cursor-pointer"
								onClick={(e) => {
									e.preventDefault();
									const newPrefix = segments.slice(0, index + 1).join("/");
									updatePrefix(newPrefix);
								}}
							>
								{part}
							</BreadcrumbLink>
						</BreadcrumbItem>
					</>
				))}
				{segments.length > 0 && (
					<>
						<BreadcrumbSeparator />
						<BreadcrumbItem>
							<BreadcrumbPage>{segments[segments.length - 1]}</BreadcrumbPage>
						</BreadcrumbItem>
					</>
				)}
			</BreadcrumbList>
		</Breadcrumb>
	);
}
