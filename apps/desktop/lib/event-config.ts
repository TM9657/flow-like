import {
	ApiConfig,
	ChatInterface,
	CronJobConfig,
	type IEventMapping,
	SimpleChatConfig,
	UserMailConfig,
	WebhookConfig,
} from "@tm9657/flow-like-ui";
import { createId } from "@paralleldrive/cuid2";

export const EVENT_CONFIG: IEventMapping = {
	events_chat: {
		configInterfaces: {
			simple_chat: SimpleChatConfig,
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
		},
		defaultEventType: "simple_chat",
		eventTypes: ["simple_chat", "advanced_chat"],
		withSink: [],
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
		},
		defaultEventType: "api",
		eventTypes: ["api"],
		configs: {
			api: {
				method: "GET",
				path: `/${createId()}`,
				public_endpoint: false,
			},
		},
		useInterfaces: {},
		withSink: ["api"],
	},
	events_simple: {
		configInterfaces: {
			webhook: WebhookConfig,
			cron: CronJobConfig
		},
		defaultEventType: "quick_action",
		eventTypes: ["quick_action", "webhook", "cron"],
		withSink: ["cron", "webhook"],
		configs: {
			cron: {
				expression: "* */1 * * *",
			},
		},
		useInterfaces: {},
	},
};
