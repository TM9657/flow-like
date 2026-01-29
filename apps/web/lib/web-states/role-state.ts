import type { IRoleState } from "@tm9657/flow-like-ui";
import type { IBackendRole } from "@tm9657/flow-like-ui/state/backend-state/types";
import { apiDelete, apiGet, apiPost, apiPut, type WebBackendRef } from "./api-utils";

export class WebRoleState implements IRoleState {
	constructor(private readonly backend: WebBackendRef) {}

	async getRoles(appId: string): Promise<[string | undefined, IBackendRole[]]> {
		try {
			return await apiGet<[string | undefined, IBackendRole[]]>(
				`apps/${appId}/roles`,
				this.backend.auth,
			);
		} catch {
			return [undefined, []];
		}
	}

	async deleteRole(appId: string, roleId: string): Promise<void> {
		await apiDelete(
			`apps/${appId}/roles/${roleId}`,
			this.backend.auth,
		);
	}

	async makeRoleDefault(appId: string, roleId: string): Promise<void> {
		await apiPut(
			`apps/${appId}/roles/${roleId}/default`,
			undefined,
			this.backend.auth,
		);
	}

	async upsertRole(appId: string, role: IBackendRole): Promise<void> {
		await apiPut(
			`apps/${appId}/roles/${role.id}`,
			role,
			this.backend.auth,
		);
	}

	async assignRole(appId: string, roleId: string, sub: string): Promise<void> {
		await apiPost(
			`apps/${appId}/roles/${roleId}/assign/${sub}`,
			undefined,
			this.backend.auth,
		);
	}
}
