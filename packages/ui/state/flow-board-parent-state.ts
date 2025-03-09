import { del, get, set } from "idb-keyval";
import { create } from "zustand";
import {
	type StateStorage,
	createJSONStorage,
	persist,
} from "zustand/middleware";

interface IFlowBoardParentState {
	boardParents: {
		[boardId: string]: string;
	};
	addBoardParent: (boardId: string, parentId: string) => void;
	removeBoardParent: (boardId: string) => void;
}

const storage: StateStorage = {
	getItem: async (name: string): Promise<string | null> => {
		console.log(name, "has been retrieved");
		return (await get(name)) ?? null;
	},
	setItem: async (name: string, value: string): Promise<void> => {
		console.log(name, "with value", value, "has been saved");
		await set(name, value);
	},
	removeItem: async (name: string): Promise<void> => {
		console.log(name, "has been deleted");
		await del(name);
	},
};

export const useFlowBoardParentState = create(
	persist<IFlowBoardParentState>(
		(set, get) => ({
			boardParents: {},
			addBoardParent: (boardId, parentLink) => {
				set((state) => {
					return {
						boardParents: {
							...state.boardParents,
							[boardId]: parentLink,
						},
					};
				});
			},
			removeBoardParent: (boardId) => {
				set((state) => {
					const newBoardParents = { ...state.boardParents };
					delete newBoardParents[boardId];
					return {
						boardParents: newBoardParents,
					};
				});
			},
		}),
		{
			name: "flow-board-parent",
			storage: createJSONStorage(() => storage),
		},
	),
);
