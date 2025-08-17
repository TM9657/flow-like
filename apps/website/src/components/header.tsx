import { Button } from "@tm9657/flow-like-ui";
import { BsDiscord, BsGithub, BsTwitterX } from "react-icons/bs";
import { LuBookHeart, LuBookMarked } from "react-icons/lu";

export function Header() {
	return (
		<header className="w-full flex flex-row items-center absolute top-0 left-0 right-0 h-16 z-20 backdrop-blur-sm shadow-md bg-background/40 justify-between">
			<a href="/" className="flex flex-row items-center px-2 gap-2">
				<img alt="logo" src="/icon.webp" className="h-12 w-12" />
				<h3 className="hidden sm:block">Flow Like</h3>
			</a>
			<div className="flex flex-row items-center px-2 gap-2">
				<a href="/blog/">
					<Button variant={"outline"}>
						<LuBookHeart width={5} height={5} className="w-5 h-5" />
						Blog
					</Button>
				</a>
				<a
					href="https://github.com/TM9657/flow-like"
					target="_blank"
					rel="noreferrer"
				>
					<Button variant={"outline"} size={"icon"}>
						<BsGithub width={5} height={5} className="w-5 h-5" />
					</Button>
				</a>
				<a href="https://x.com/greadco_de" target="_blank" rel="noreferrer">
					<Button variant={"outline"} size={"icon"}>
						<BsTwitterX width={5} height={5} className="w-5 h-5" />
					</Button>
				</a>
				<a
					href="https://discord.com/invite/KTWMrS2/"
					target="_blank"
					rel="noreferrer"
				>
					<Button variant={"outline"} size={"icon"}>
						<BsDiscord width={5} height={5} className="w-5 h-5" />
					</Button>
				</a>
				<a href="https://docs.flow-like.com" target="_blank" rel="noreferrer">
					<Button>
						<LuBookMarked className="w-5 h-5" />
						Docs
					</Button>
				</a>
			</div>
		</header>
	);
}
