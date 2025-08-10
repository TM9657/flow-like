import { useCallback, useEffect, useState } from "react";
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
import { useBackend } from "../../state/backend-state";
import { Badge } from "./badge";
import { BitTypeIcon, bitTypeToText } from "./bit-card";

export function BitHover({
	bit,
	children,
}: Readonly<{ bit: IBit; children: React.ReactNode }>) {
	const backend = useBackend();
	const [bitData] = useState<Bit>(
		bit instanceof Bit ? bit : Bit.fromObject(bit),
	);
	const [bitSize, setBitSize] = useState<number>(0);

	const fetchSize = useCallback(async () => {
		if (!backend) return;
		if (!bitData.backend) {
			bitData.setBackend(backend);
		}
		const size = await bitData.fetchSize();
		setBitSize(size);
	}, [bitData, backend]);

	useEffect(() => {
		if (!backend) return;
		fetchSize();
	}, [bitData, backend]);

	return (
		<HoverCard>
			<HoverCardTrigger>{children}</HoverCardTrigger>
			<HoverCardContent className="z-50">
				<div className="flex flex-row items-center gap-2">
					<Avatar className="border bg-card z-10 overflow-hidden">
						<AvatarImage
							className="p-1 transition-transform duration-200 ease-in-out transform scale-110 group-hover:scale-150 rounded-full"
							src={bit.meta?.en?.icon ?? "/app-logo.webp"}
						/>
						<AvatarFallback>NA</AvatarFallback>
					</Avatar>
					<div>
						<div className="flex flex-row items-center gap-1">
							<h4 className="text-sm text-start">{bitData.meta?.en?.name}</h4>
							<BitTypeIcon type={bitData.type} className="w-3 h-3" />
						</div>
						<p className="text-xs text-muted-foreground text-start">
							{bitTypeToText(bitData.type)}
						</p>
					</div>
				</div>
				<br />
				<p className="text-xs text-start line-clamp-3">
					{bitData.meta?.en?.description}
				</p>
				<br />
				{"languages" in bitData.parameters && (
					<div className="flex flex-row items-center gap-2 mb-2">
						{bitData.parameters.languages.map((lang: string) => (
							<Badge variant={"outline"} key={lang}>
								{lang}
							</Badge>
						))}
					</div>
				)}
				<div className="flex flex-row items-center justify-start">
					<Badge>{humanFileSize(bitSize)}</Badge>
				</div>
			</HoverCardContent>
		</HoverCard>
	);
}
