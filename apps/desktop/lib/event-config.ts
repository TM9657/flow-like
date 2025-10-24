import { createId } from "@paralleldrive/cuid2";
import {
	ApiConfig,
	ChatInterface,
	CronJobConfig,
	DeeplinkConfig,
	DiscordConfig,
	type IEventMapping,
	SimpleChatConfig,
	UserMailConfig,
	WebhookConfig,
} from "@tm9657/flow-like-ui";

export const EVENT_CONFIG: IEventMapping = {
	events_chat: {
		configInterfaces: {
			simple_chat: SimpleChatConfig,
			discord: DiscordConfig,
		},
		useInterfaces: {
			simple_chat: ChatInterface,
		},
		configs: {
			simple_chat: {
				allow_file_upload: true,
				allow_voice_input: false,
				history_elements: 5,
				tools: [],
				default_tools: [],
				example_messages: [],
			},
			discord: {
				token: "",
				bot_name: "Flow-Like Bot",
				bot_description: "",
				intents: ["Guilds", "GuildMessages", "MessageContent"],
				channel_whitelist: [],
				channel_blacklist: [],
				respond_to_mentions: true,
				respond_to_dms: true,
				command_prefix: "!",
			},
		},
		defaultEventType: "simple_chat",
		eventTypes: ["simple_chat", "advanced_chat", "discord"],
		withSink: ["discord"],
	},
	events_mail: {
		configInterfaces: {
			user_mail: UserMailConfig,
		},
		defaultEventType: "email",
		eventTypes: ["email"],
		configs: {
			email: {
				imap_server: "",
				imap_port: 993,
				username: "",
				password: "",
				use_tls: true,
			},
		},
		useInterfaces: {},
		withSink: ["email"],
	},
	events_generic: {
		configInterfaces: {
			api: ApiConfig,
			deeplink: DeeplinkConfig,
		},
		defaultEventType: "api",
		eventTypes: ["api", "deeplink"],
		configs: {
			api: {
				method: "GET",
				path: `/${createId()}`,
				public_endpoint: false,
			},
			deeplink: {
				path: createId(),
			},
		},
		useInterfaces: {},
		withSink: ["api", "deeplink"],
	},
	events_simple: {
		configInterfaces: {
			api: WebhookConfig,
			cron: CronJobConfig,
			deeplink: DeeplinkConfig,
		},
		defaultEventType: "quick_action",
		eventTypes: ["quick_action", "api", "cron", "deeplink"],
		withSink: ["cron", "api", "deeplink"],
		configs: {
			cron: {
				expression: "* */1 * * *",
			},
			deeplink: {
				path: createId(),
			},
		},
		useInterfaces: {},
	},
};
