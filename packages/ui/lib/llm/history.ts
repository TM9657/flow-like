import { type Nullable } from "../schema/auto-import";
import {
	type IHistory,
	type IStreamOptions,
	type IHistoryMessage,
	type ITool,
	IContentType,
	IRole,
	type IToolChoice,
} from "../schema/llm/history";

export class History implements IHistory {
	messages: IHistoryMessage[] = [];
	model: string = "";
	stream: Nullable<boolean>;
	streamOptions: Nullable<IStreamOptions>;
	maxCompletionTokens: Nullable<number>;
	topP: Nullable<number>;
	temperature: Nullable<number>;
	seed: Nullable<number>;
	presencePenalty: Nullable<number>;
	frequencyPenalty: Nullable<number>;
	user: Nullable<string>;
	stop: Nullable<string[]>;
	responseFormat: any;
	n: Nullable<number>;
	tools: Nullable<ITool[]>;
	toolChoice: Nullable<IToolChoice>;

	public static fromJson(json: string): History {
		const object = JSON.parse(json);
		return History.fromObject(object);
	}

	public toJson(): string {
		const object = this.toObject();
		return JSON.stringify(object);
	}

	public static fromObject(obj: IHistory): History {
		const history = new History();

		for (const key of Object.keys(obj)) {
			(history as any)[key] = obj[key];
		}

		return history;
	}

	public toObject(): IHistory {
		const obj: Record<string, any> = {};
		Object.keys(this).forEach((key) => {
			if (typeof (this as any)[key] !== "function") {
				obj[key] = (this as any)[key];
			}
		});
		return obj as IHistory;
	}

	pushMessage(message: IHistoryMessage) {
		this.messages.push(message);
	}

	setSystemPrompt(prompt: string) {
		const system_prompt_index = this.messages.findIndex(
			(message) => message.role === "system",
		);
		if (system_prompt_index !== -1) {
			this.messages[system_prompt_index].content = [
				{
					type: IContentType.Text,
					text: prompt,
				},
			];

			return;
		}

		this.messages.unshift({
			role: IRole.System,
			content: [{ type: IContentType.Text, text: prompt }],
		});
	}

	setStream(stream: boolean) {
		this.stream = stream;
	}
}
