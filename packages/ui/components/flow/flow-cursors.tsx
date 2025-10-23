"use client";
import { memo, useEffect, useMemo, useState } from "react";

interface CursorState {
  x: number;
  y: number;
  name?: string;
  color?: string;
  id?: string;
}

export const FlowCursors = memo(function FlowCursors({ awareness, className }: { awareness?: any; className?: string }) {
  const [peers, setPeers] = useState<Map<number, CursorState>>(new Map());

  const update = () => {
    if (!awareness) return;
    const states = awareness.getStates() as Map<number, any>;
    const invalid: Set<number> | undefined = (awareness as any)?.__invalidPeers;
    const next = new Map<number, CursorState>();
    states.forEach((s, clientId) => {
      const c = s?.cursor;
      const u = s?.user;
      if (!c || clientId === awareness.clientID) return;
      // Skip invalid peers if validation failed
      if (invalid && invalid.has(clientId)) return;
      next.set(clientId, {
        x: c.x,
        y: c.y,
        name: u?.name ?? "User",
        color: u?.color ?? "#22c55e",
        id: u?.id,
      });
    });
    setPeers(next);
  };

  useEffect(() => {
    if (!awareness) return;
    const onChange = () => update();
    awareness.on("change", onChange);
    update();
    return () => {
      try { awareness.off("change", onChange); } catch {}
    };
  }, [awareness]);

  const items = useMemo(() => Array.from(peers.values()), [peers]);

  return (
    <div className={"pointer-events-none absolute inset-0 z-40 " + (className ?? "")}>
        <div className="size-10 bg-red-500 top-0 right-0 z-10"></div>
      {items.map((p, idx) => (
        <div key={(p.id ?? "") + idx} className="absolute" style={{ transform: `translate(${p.x}px, ${p.y}px)` }}>
          <div className="flex items-center gap-1">
            <div className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: p.color }} />
            <div className="text-xs px-1.5 py-0.5 rounded bg-[color-mix(in_oklch,var(--background)_60%,transparent)] border text-foreground" style={{ borderColor: "color-mix(in oklch, var(--foreground) 10%, transparent)" }}>
              {p.name}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
});
