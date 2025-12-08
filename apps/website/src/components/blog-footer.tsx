export function BlogFooter() {
	return (
		<footer className="w-full flex flex-row items-center h-10 z-20 bg-transparent justify-between px-2 gap-2">
			<div>
				<small>© 2025 Flow-Like - Made with ❤️ in Munich</small>
			</div>
			<div className="flex flex-row items-center gap-2">
				<a href="/eula">
					<small>EULA</small>
				</a>
				<a href="/privacy-policy">
					<small>Privacy Policy</small>
				</a>
				<a
					href="https://great-co.de/legal-notice"
					target="_blank"
					rel="noreferrer"
				>
					<small>Legal Notice</small>
				</a>
			</div>
		</footer>
	);
}
