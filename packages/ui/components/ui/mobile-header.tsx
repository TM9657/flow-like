"use client";
import { createId } from "@paralleldrive/cuid2";
import {
  SidebarTrigger,
} from "@tm9657/flow-like-ui";
import React, {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  useEffect,
} from "react";

export type MobileHeaderControls = {
  title?: React.ReactNode;
  left?: React.ReactNode | React.ReactNode[];
  right?: React.ReactNode | React.ReactNode[];
};

type Ctx = {
  register: (id: string, controls: MobileHeaderControls) => void;
  update: (id: string, controls: MobileHeaderControls) => void;
  unregister: (id: string) => void;
  active: MobileHeaderControls | null;
};

const MobileHeaderContext = createContext<Ctx | null>(null);

export const MobileHeaderProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const [controlsMap, setControlsMap] = useState<Map<string, MobileHeaderControls>>(new Map());

  const register = useCallback((id: string, controls: MobileHeaderControls) => {
    setControlsMap((prev) => {
      const next = new Map(prev);
      next.set(id, controls);
      return next;
    });
  }, []);

  const update = useCallback((id: string, controls: MobileHeaderControls) => {
    setControlsMap((prev) => {
      const next = new Map(prev);
      const existing = next.get(id) ?? {};
      next.set(id, { ...existing, ...controls });
      return next;
    });
  }, []);

  const unregister = useCallback((id: string) => {
    setControlsMap((prev) => {
      if (!prev.has(id)) return prev;
      const next = new Map(prev);
      next.delete(id);
      return next;
    });
  }, []);

  const active = useMemo<MobileHeaderControls | null>(() => {
    if (controlsMap.size === 0) return null;
    const last = Array.from(controlsMap.values()).at(-1) ?? null;
    return last ?? null;
  }, [controlsMap]);

  const value = useMemo<Ctx>(() => ({ register, update, unregister, active }), [register, update, unregister, active]);

  return (
    <MobileHeaderContext.Provider value={value}>{children}</MobileHeaderContext.Provider>
  );
};

export function useMobileHeader(controls?: MobileHeaderControls, deps: React.DependencyList = []) {
  const ctx = useContext(MobileHeaderContext);
  if (!ctx) throw new Error("useMobileHeader must be used within MobileHeaderProvider");
  const idRef = useRef<string | null>(null);

  const ensureId = useCallback(() => {
  if (!idRef.current) idRef.current = createId();
  return idRef.current;
  }, []);

  useEffect(() => {
    if (!controls) return;
    const id = ensureId();
    ctx.register(id, controls);
    return () => ctx.unregister(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  const set = useCallback(
    (next: MobileHeaderControls) => {
      const id = ensureId();
      ctx.register(id, next);
      return () => ctx.unregister(id);
    },
    [ctx, ensureId],
  );

  const update = useCallback(
    (next: MobileHeaderControls) => {
      const id = ensureId();
      ctx.update(id, next);
    },
    [ctx, ensureId],
  );

  const clear = useCallback(() => {
    if (!idRef.current) return;
    ctx.unregister(idRef.current);
  }, [ctx]);

  return { set, update, clear } as const;
}

export const MobileHeader: React.FC = () => {
  const ctx = useContext(MobileHeaderContext);
  const active = ctx?.active ?? null;

  const left = useMemo(() => {
    if (!active?.left) return null;
    return Array.isArray(active.left) ? active.left : [active.left];
  }, [active?.left]);

  const right = useMemo(() => {
    if (!active?.right) return null;
    return Array.isArray(active.right) ? active.right : [active.right];
  }, [active?.right]);

  return (
    <div className="md:hidden sticky top-0 z-40 px-2 bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="flex items-center justify-between gap-2 p-2 rounded-xl bg-card/80 shadow-2xl">
        <div className="flex items-center gap-2 min-w-0">
          <SidebarTrigger className="size-9 rounded-lg border" aria-label="Open Menu" />
          {left?.map((node, i) => (
            <React.Fragment key={i}>{node}</React.Fragment>
          ))}
        </div>
        <div className="flex-1 min-w-0 text-center font-medium truncate">
          {active?.title ?? null}
        </div>
        <div className="flex items-center gap-2">
          {right?.map((node, i) => (
            <React.Fragment key={i}>{node}</React.Fragment>
          ))}
        </div>
      </div>
    </div>
  );
};
