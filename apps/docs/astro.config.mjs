import react from "@astrojs/react";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";

import { defineConfig, passthroughImageService } from "astro/config";
// https://astro.build/config
export default defineConfig({
	site: "https://docs.flow-like.com",
	output: "static",
	image: {
		service: passthroughImageService(),
	},

	integrations: [
		react(),
		starlight({
			title: "Flow-Like",
			favicon: "/ico-light.svg",
			description:
				"Build typed, scalable workflows with visual design, Rust performance, and production-ready governance.",
			components: {
				Hero: "./src/components/docs/Hero.astro",
			},
			head: [
				{
					tag: "link",
					attrs: {
						rel: "icon",
						href: "/ico.ico",
						sizes: "32x32",
					},
				},
				{
					tag: "script",
					attrs: { id: "posthog", type: "text/javascript" },
					content: `
!function(t,e){var o,n,p,r;e.__SV||(window.posthog=e,e._i=[],e.init=function(i,s,a){function g(t,e){var o=e.split(".");2==o.length&&(t=t[o[0]],e=o[1]),t[e]=function(){t.push([e].concat(Array.prototype.slice.call(arguments,0)))}}(p=t.createElement("script")).type="text/javascript",p.crossOrigin="anonymous",p.async=!0,p.src=s.api_host.replace(".i.posthog.com","-assets.i.posthog.com")+"/static/array.js",(r=t.getElementsByTagName("script")[0]).parentNode.insertBefore(p,r);var u=e;for(void 0!==a?u=e[a]=[]:a="posthog",u.people=u.people||[],u.toString=function(t){var e="posthog";return"posthog"!==a&&(e+="."+a),t||(e+=" (stub)"),e},u.people.toString=function(){return u.toString(1)+".people (stub)"},o="init Ce Os As Te Cs Fs capture Ye calculateEventProperties Ls register register_once register_for_session unregister unregister_for_session qs getFeatureFlag getFeatureFlagPayload isFeatureEnabled reloadFeatureFlags updateEarlyAccessFeatureEnrollment getEarlyAccessFeatures on onFeatureFlags onSurveysLoaded onSessionId getSurveys getActiveMatchingSurveys renderSurvey canRenderSurvey canRenderSurveyAsync identify setPersonProperties group resetGroups setPersonPropertiesForFlags resetPersonPropertiesForFlags setGroupPropertiesForFlags resetGroupPropertiesForFlags reset get_distinct_id getGroups get_session_id get_session_replay_url alias set_config startSessionRecording stopSessionRecording sessionRecordingStarted captureException loadToolbar get_property getSessionProperty zs js createPersonProfile Us Rs Bs opt_in_capturing opt_out_capturing has_opted_in_capturing has_opted_out_capturing get_explicit_consent_status is_capturing clear_opt_in_out_capturing Ds debug L Ns getPageViewId captureTraceFeedback captureTraceMetric".split(" "),n=0;n<o.length;n++)g(u,o[n]);e._i.push([i,s,a])},e.__SV=1)}(document,window.posthog||[]);
posthog.init('phc_hxGZEJaPqyCNzqqfrYyuUDCUSpcc7RSbwh07t4xtfrE', { api_host:'https://eu.i.posthog.com', autocapture:true, capture_pageview:true, person_profiles:'identified_only' });
          `.trim(),
				},
			],
			editLink: {
				baseUrl: "https://github.com/TM9657/flow-like/edit/main/apps/docs/",
			},
			logo: {
				light: "./src/assets/app-logo-light.webp",
				dark: "./src/assets/app-logo.webp",
			},
			customCss: ["./src/styles/global.css"],
			social: [
				{
					icon: "discord",
					label: "Discord",
					href: "https://discord.gg/KTWMrS2",
				},
				{
					icon: "github",
					label: "GitHub",
					href: "https://github.com/TM9657/flow-like",
				},
				{ icon: "x.com", label: "X.com", href: "https://x.com/greatco_de" },
				{
					icon: "linkedin",
					label: "LinkedIn",
					href: "https://linkedin.com/company/greatco-de",
				},
			],
			lastUpdated: true,
			tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 4 },
			sidebar: [
				{
					label: "Getting Started",
					items: [
						{ label: "Quick Start", slug: "start/getting-started" },
						{ label: "What is Flow-Like?", slug: "start/what-is-flow-like" },
						{ label: "Download & Install", slug: "start/get" },
						{ label: "First Steps", slug: "start/first-use" },
						{ label: "Login & Accounts", slug: "start/login" },
						{ label: "AI Models", slug: "start/models" },
						{ label: "Profiles", slug: "start/profiles" },
						{ label: "Get Support", slug: "start/support" },
					],
				},
				{
					label: "Studio",
					items: [
						{ label: "Overview", slug: "studio/overview" },
						{ label: "Working with Nodes", slug: "studio/nodes" },
						{ label: "Connecting Pins", slug: "studio/connecting" },
						{ label: "Layers & Organization", slug: "studio/layers" },
						{ label: "Variables", slug: "studio/variables" },
						{ label: "Logging & Debugging", slug: "studio/logging" },
						{ label: "Version Control", slug: "studio/versioning" },
					],
				},
				{
					label: "Apps",
					items: [
						{ label: "Overview", slug: "apps/overview" },
						{ label: "Creating Apps", slug: "apps/create" },
						{ label: "Boards & Flows", slug: "apps/boards" },
						{ label: "Chat UI", slug: "apps/chat-ui" },
						{ label: "Templates", slug: "apps/templates" },
						{ label: "Events", slug: "apps/events" },
						{ label: "Storage", slug: "apps/storage" },
						{ label: "Sharing", slug: "apps/share" },
						{ label: "Offline & Online", slug: "apps/offline-online" },
					],
				},
				{
					label: "Self Hosting",
					items: [
						{ label: "Overview", slug: "self-hosting/overview" },
						{
							label: "Kubernetes",
							collapsed: true,
							autogenerate: { directory: "self-hosting/kubernetes" },
						},
					],
				},
				{
					label: "For Developers",
					items: [
						{ label: "Architecture", slug: "dev/architecture" },
						{ label: "Building from Source", slug: "dev/build" },
						{ label: "Contributing", slug: "dev/contribute" },
						{ label: "Writing Custom Nodes", slug: "dev/writing-nodes" },
						{ label: "Rust SDK", slug: "dev/rust" },
						{ label: "Storage Providers", slug: "dev/storage-providers" },
						{ label: "Customization", slug: "dev/customizing" },
					],
				},
				{
					label: "For Enterprises",
					autogenerate: { directory: "enterprise" },
				},
				{
					label: "Node Catalog",
					items: [
						{ label: "Overview", slug: "nodes/overview" },
						{
							label: "AI & LLM",
							collapsed: true,
							autogenerate: { directory: "nodes/AI" },
						},
						{
							label: "Control Flow",
							collapsed: true,
							autogenerate: { directory: "nodes/Control" },
						},
						{
							label: "Database",
							collapsed: true,
							autogenerate: { directory: "nodes/Database" },
						},
						{
							label: "Events",
							collapsed: true,
							autogenerate: { directory: "nodes/Events" },
						},
						{
							label: "Image Processing",
							collapsed: true,
							autogenerate: { directory: "nodes/Image" },
						},
						{
							label: "Logging",
							collapsed: true,
							autogenerate: { directory: "nodes/Logging" },
						},
						{
							label: "Math",
							collapsed: true,
							autogenerate: { directory: "nodes/Math" },
						},
						{
							label: "Storage",
							collapsed: true,
							autogenerate: { directory: "nodes/Storage" },
						},
						{
							label: "Data Structures",
							collapsed: true,
							autogenerate: { directory: "nodes/Structs" },
						},
						{
							label: "Utilities",
							collapsed: true,
							autogenerate: { directory: "nodes/Utils" },
						},
						{
							label: "Variables",
							collapsed: true,
							autogenerate: { directory: "nodes/Variable" },
						},
						{
							label: "Web & HTTP",
							collapsed: true,
							autogenerate: { directory: "nodes/Web" },
						},
						{
							label: "Bit Operations",
							collapsed: true,
							autogenerate: { directory: "nodes/Bit" },
						},
					],
				},
			],
		}),
	],
	vite: {
		ssr: {
			noExternal: [
				"katex",
				"rehype-katex",
				"@tm9657/flow-like-ui",
				"lodash-es",
				"@platejs/math",
				"react-lite-youtube-embed",
				"react-tweet",
			],
		},
		define: {
			"process.env": {},
			"process.env.NODE_ENV": JSON.stringify(
				process.env.NODE_ENV || "production",
			),
		},
		plugins: [tailwindcss()],
	},
});
