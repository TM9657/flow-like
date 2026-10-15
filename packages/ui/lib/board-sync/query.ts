import {
	type QueryClient,
	type QueryKey,
	replaceEqualDeep,
} from "@tanstack/react-query";
import { isBoardSnapshot } from "./identity";

/** Keep sync identities; persisted and legacy responses still use normal cache merging. */
export function shareBoardSnapshot(previous: unknown, next: unknown): unknown {
	return isBoardSnapshot(next) ? next : replaceEqualDeep(previous, next);
}

export function setSnapshotQueryData<T>(
	client: QueryClient,
	queryKey: QueryKey,
	data: T,
): void {
	if (isBoardSnapshot(data)) {
		// Command responses can arrive before a query observer mounts.
		const query = client.getQueryCache().build(client, { queryKey });
		query.setOptions({
			...query.options,
			structuralSharing: shareBoardSnapshot,
		});
	}
	client.setQueryData(queryKey, data);
}
