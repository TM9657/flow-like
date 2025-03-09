import { useEffect, useState } from "react";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
} from "../../components/ui/avatar";
import {
	HoverCard,
	HoverCardContent,
	HoverCardTrigger,
} from "../../components/ui/hover-card";
import { Bit } from "../../lib/bit/bit";
import type { IBit } from "../../lib/schema/bit/bit";
import { humanFileSize } from "../../lib/utils";
import { Badge } from "./badge";

export function BitHover({
	bit,
	children,
}: Readonly<{ bit: IBit; children: React.ReactNode }>) {
	const [bitData] = useState<Bit>(
		bit instanceof Bit ? bit : Bit.fromObject(bit),
	);
	const [bitSize, setBitSize] = useState<number>(0);

	async function fetchSize() {
		const size = await bitData.fetchSize();
		setBitSize(size);
	}

	useEffect(() => {
		fetchSize();
	}, [bitData]);

	return (
		<HoverCard>
			<HoverCardTrigger>{children}</HoverCardTrigger>
			<HoverCardContent>
				<div className="flex flex-row items-center gap-2">
					<Avatar className="border">
						<AvatarImage className="p-1" src={bitData.icon} />
						<AvatarImage className="p-1" src="/app-logo.webp" />
						<AvatarFallback>NA</AvatarFallback>
					</Avatar>
					<div>
						<h4 className="text-sm">{bitData.meta.en.name}</h4>
						<p className="text-xs text-muted-foreground">{bitData.type}</p>
					</div>
				</div>
				<br />
				<p className="text-xs line-clamp-3">{bitData.meta.en.description}</p>
				<br />
				<div className="flex flex-row items-center gap-2">
					{"languages" in bitData.parameters &&
						bitData.parameters.languages.map((lang: string) => (
							<Badge key={lang}>{lang}</Badge>
						))}
					<Badge>{humanFileSize(bitSize)}</Badge>
				</div>
			</HoverCardContent>
		</HoverCard>
	);
}
