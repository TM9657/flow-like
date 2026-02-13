import Dexie, { type EntityTable } from "dexie";

export interface ITemporaryFiles {
	id: string;
	fileName: string;
	size: number;
	hash: string;
	createdAt: number;
}

const temporaryFilesDb = new Dexie("Temporary-Files-DB") as Dexie & {
	temporaryFiles: EntityTable<ITemporaryFiles, "id">;
};

temporaryFilesDb.version(1).stores({
	temporaryFiles: "++id, hash, fileName, size, createdAt",
});

export { temporaryFilesDb };
