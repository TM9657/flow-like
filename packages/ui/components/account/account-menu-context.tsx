"use client";

import { type ComponentType, createContext, useContext } from "react";

// Hosts supply their account actions so the board keeps the same menu and session.
const AccountMenuContext = createContext<ComponentType<{
	compact?: boolean;
}> | null>(null);

export const AccountMenuProvider = AccountMenuContext.Provider;
export const useAccountMenu = () => useContext(AccountMenuContext);
