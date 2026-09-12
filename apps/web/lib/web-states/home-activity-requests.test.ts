import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiGet: vi.fn() }));

vi.mock("@flow-like/flow-like-ui", () => ({}));
vi.mock("./api-utils", () => ({ apiGet: mocks.apiGet }));
vi.mock("../oauth-db", () => ({ oauthConsentStore: {}, oauthTokenStore: {} }));
vi.mock("../oauth-service", () => ({}));
vi.mock("sonner", () => ({ toast: vi.fn() }));

import { WebEventState } from "./event-state";
import { WebUsageState } from "./usage-state";
import { WebUserState } from "./user-state";

// The notification reader refuses to call the hub without an access token, so
// the stub carries one.
const auth = { isAuthenticated: true, user: { access_token: "token" } };
const backend = { auth } as never;

/** The path a call was made with, so a typo cannot pass as a silent 404. */
const path = () => String(mocks.apiGet.mock.calls[0][0]);

describe("home widgets ask the source for whole periods, not pages", () => {
	beforeEach(() => vi.resetAllMocks());

	test("execution activity carries the window and the optional app", async () => {
		mocks.apiGet.mockResolvedValue({ total: 4861 });
		await new WebUsageState(backend).getExecutionActivity(30, "app-a");
		expect(path()).toBe("usage/executions/activity?days=30&app_id=app-a");
		expect(mocks.apiGet).toHaveBeenCalledWith(expect.any(String), auth);
	});

	test("execution activity defaults to a week and omits an absent app", async () => {
		mocks.apiGet.mockResolvedValue({ total: 0 });
		await new WebUsageState(backend).getExecutionActivity();
		expect(path()).toBe("usage/executions/activity?days=7");
	});

	test("a notification kind is sent to the source rather than filtered locally", async () => {
		mocks.apiGet.mockResolvedValue([]);
		await new WebUserState(backend).listNotifications(true, "WORKFLOW", 0, 8);
		expect(path()).toContain("notification_type=WORKFLOW");
		expect(path()).toContain("limit=8");
		expect(path()).toContain("unread_only=true");
	});

	test("omitting the kind asks for every kind", async () => {
		mocks.apiGet.mockResolvedValue([]);
		await new WebUserState(backend).listNotifications(false, undefined, 0, 8);
		expect(path()).not.toContain("notification_type");
	});

	test("schedules are read once for the account, not once per app", async () => {
		mocks.apiGet.mockResolvedValue({ schedules: [], apps_checked: 0 });
		await new WebEventState(backend).getUserSchedules(25);
		expect(path()).toBe("user/schedules?limit=25");
		expect(mocks.apiGet).toHaveBeenCalledTimes(1);
	});
});
