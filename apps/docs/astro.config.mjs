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
				"Flow-Like is a visual programming language for creating very fast and efficient workflows and automations.",
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
					tag: 'script',
					attrs: { id: 'posthog', type: 'text/javascript' },
					content: `
!function(t,e){var o,n,p,r;e.__SV||(window.posthog=e,e._i=[],e.init=function(i,s,a){function g(t,e){var o=e.split(".");2==o.length&&(t=t[o[0]],e=o[1]),t[e]=function(){t.push([e].concat(Array.prototype.slice.call(arguments,0)))}}(p=t.createElement("script")).type="text/javascript",p.crossOrigin="anonymous",p.async=!0,p.src=s.api_host.replace(".i.posthog.com","-assets.i.posthog.com")+"/static/array.js",(r=t.getElementsByTagName("script")[0]).parentNode.insertBefore(p,r);var u=e;for(void 0!==a?u=e[a]=[]:a="posthog",u.people=u.people||[],u.toString=function(t){var e="posthog";return"posthog"!==a&&(e+="."+a),t||(e+=" (stub)"),e},u.people.toString=function(){return u.toString(1)+".people (stub)"},o="init Ce Os As Te Cs Fs capture Ye calculateEventProperties Ls register register_once register_for_session unregister unregister_for_session qs getFeatureFlag getFeatureFlagPayload isFeatureEnabled reloadFeatureFlags updateEarlyAccessFeatureEnrollment getEarlyAccessFeatures on onFeatureFlags onSurveysLoaded onSessionId getSurveys getActiveMatchingSurveys renderSurvey canRenderSurvey canRenderSurveyAsync identify setPersonProperties group resetGroups setPersonPropertiesForFlags resetPersonPropertiesForFlags setGroupPropertiesForFlags resetGroupPropertiesForFlags reset get_distinct_id getGroups get_session_id get_session_replay_url alias set_config startSessionRecording stopSessionRecording sessionRecordingStarted captureException loadToolbar get_property getSessionProperty zs js createPersonProfile Us Rs Bs opt_in_capturing opt_out_capturing has_opted_in_capturing has_opted_out_capturing get_explicit_consent_status is_capturing clear_opt_in_out_capturing Ds debug L Ns getPageViewId captureTraceFeedback captureTraceMetric".split(" "),n=0;n<o.length;n++)g(u,o[n]);e._i.push([i,s,a])},e.__SV=1)}(document,window.posthog||[]);
posthog.init('phc_hxGZEJaPqyCNzqqfrYyuUDCUSpcc7RSbwh07t4xtfrE', { api_host:'https://eu.i.posthog.com', autocapture:true, capture_pageview:true, person_profiles:'identified_only' });
          `.trim(),
				}
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
			sidebar: [
				{
					label: "Guides",
					autogenerate: { directory: "guides" },
				},
				{
					label: "Contributing",
					autogenerate: { directory: "contributing" },
				},
				{
					label: "Nodes",
					autogenerate: { directory: "nodes" },
				},
				{
					label: "Reference",
					autogenerate: { directory: "reference" },
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
			// stub out `process.env` so next/dist/client code can import it without blowing up
			"process.env": {},
			// if any code reads process.env.NODE_ENV, you can explicitly set it:
			"process.env.NODE_ENV": JSON.stringify(
				process.env.NODE_ENV || "production",
			),
		},
		plugins: [

			tailwindcss(),
		],
	},
});
