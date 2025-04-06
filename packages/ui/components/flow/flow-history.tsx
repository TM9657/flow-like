import Dexie from "dexie";
import type { IGenericCommand } from "../../lib";

interface IStackItem {
	key: string;
	undoStack: IGenericCommand[][];
	redoStack: IGenericCommand[][];
}

class UndoRedoDB extends Dexie {
	stacks!: Dexie.Table<IStackItem, string>;

	constructor() {
		super("undo-redo");
		this.version(1).stores({
			stacks: "key",
		});
	}
}

const db = new UndoRedoDB();

const MAX_STACK_SIZE = 100;

export const useUndoRedo = (appId: string, boardId: string) => {
	const key = `${appId}_${boardId}`;

	const pushCommand = async (command: IGenericCommand, append = false) => {
		await db.transaction("rw", db.stacks, async () => {
			const data = await db.stacks.get(key);
			const currentUndoStack = data?.undoStack || [];
			let newUndoStack;

			if (append && currentUndoStack.length > 0) {
				const lastBatch = currentUndoStack[currentUndoStack.length - 1];
				newUndoStack = [
					...currentUndoStack.slice(0, -1),
					[...lastBatch, command],
				];
			} else {
				newUndoStack = [...currentUndoStack, [command]];
			}

			if (newUndoStack.length > MAX_STACK_SIZE) {
				newUndoStack = newUndoStack.slice(1);
			}

			await db.stacks.put({
				key,
				undoStack: newUndoStack,
				redoStack: [],
			});
		});
	};

	const pushCommands = async (commands: IGenericCommand[]) => {
		await db.transaction("rw", db.stacks, async () => {
			const data = await db.stacks.get(key);
			const currentUndoStack = data?.undoStack || [];
			let newUndoStack = [...currentUndoStack, commands];

			if (newUndoStack.length > MAX_STACK_SIZE) {
				newUndoStack = newUndoStack.slice(1);
			}

			await db.stacks.put({
				key,
				undoStack: newUndoStack,
				redoStack: [],
			});
		});
	};

	const undo = async () => {
		return await db.transaction("rw", db.stacks, async () => {
			const data = await db.stacks.get(key);
			const currentUndoStack = data?.undoStack || [];
			if (currentUndoStack.length === 0) return null;

			const lastBatch = currentUndoStack[currentUndoStack.length - 1];
			const newUndoStack = currentUndoStack.slice(0, -1);
			const currentRedoStack = data?.redoStack || [];
			let newRedoStack = [lastBatch, ...currentRedoStack];

			if (newRedoStack.length > MAX_STACK_SIZE) {
				newRedoStack = newRedoStack.slice(0, MAX_STACK_SIZE);
			}

			await db.stacks.put({
				key,
				undoStack: newUndoStack,
				redoStack: newRedoStack,
			});

			return lastBatch;
		});
	};

	const redo = async () => {
		return await db.transaction("rw", db.stacks, async () => {
			const data = await db.stacks.get(key);
			const currentRedoStack = data?.redoStack || [];
			if (currentRedoStack.length === 0) return null;

			const firstBatch = currentRedoStack[0];
			const newRedoStack = currentRedoStack.slice(1);
			const currentUndoStack = data?.undoStack || [];
			let newUndoStack = [...currentUndoStack, firstBatch];

			if (newUndoStack.length > MAX_STACK_SIZE) {
				newUndoStack = newUndoStack.slice(1);
			}

			await db.stacks.put({
				key,
				undoStack: newUndoStack,
				redoStack: newRedoStack,
			});

			return firstBatch;
		});
	};

	return { pushCommand, pushCommands, undo, redo };
};
