export function Footer() {
	return (
		<footer className="w-full flex flex-row items-center absolute bottom-0 left-0 right-0 h-10 z-20 bg-transparent justify-start px-2 gap-2">
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
		</footer>
	);
}
