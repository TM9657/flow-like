import { jwtVerify, importJWK, type JWK } from "jose";
import type { IJwks, IRealtimeAccess } from "./types";
import * as Y from 'yjs'
import { WebrtcProvider } from 'y-webrtc'

export interface RealtimeSession {
  doc: Y.Doc;
  provider: any; // WebrtcProvider, typed as any to avoid direct dependency types here
  awareness: any;
  dispose: () => void;
}

function pickColor(seed?: string): string {
  // Stable HSL color from seed, fallback random
  let s = 0;
  const str = seed ?? Math.random().toString(36);
  for (let i = 0; i < str.length; i++) s = (s * 31 + str.charCodeAt(i)) >>> 0;
  const hue = s % 360;
  return `hsl(${hue} 80% 50%)`;
}

async function verifyPeerJwt(token: string, jwks: IJwks): Promise<boolean> {
  try {
    // Resolve key by kid if present, else try first EC key
    const [header] = token.split(".");
    const kid = (() => {
      try {
        const json = JSON.parse(atob(header.replace(/-/g, "+").replace(/_/g, "/")));
        return json.kid as string | undefined;
      } catch {
        return undefined;
      }
    })();

    let key: JWK | undefined = jwks.keys.find((k) => (kid ? k.kid === kid : k.kty === "EC"));
    if (!key) return false;

    const cryptoKey = await importJWK(key as JWK, key.alg || "ES256");
    await jwtVerify(token, cryptoKey, {
      algorithms: ["ES256"],
      // We accept iss/aud validation at the backend; here we only verify signature
    });
    return true;
  } catch {
    return false;
  }
}

export async function createRealtimeSession(args: {
  room: string;
  access: IRealtimeAccess;
  jwks?: IJwks;
  name?: string;
  userId?: string;
  signalingServers?: string[];
}): Promise<RealtimeSession> {
  const { room, access, jwks, name, userId } = args;
  const doc = new Y.Doc();
  if(!args.signalingServers){
    console.warn("No signaling servers provided, using default");
  } else {
    console.log("Using signaling servers:", args.signalingServers);
  }

  const provider = new WebrtcProvider(room, doc, {
    password: access.encryption_key,
    maxConns: 20 + Math.floor(Math.random() * 15),
    signaling: args.signalingServers ?? [
        'wss://signaling.flow-like.com'
    ],
    filterBcConns: true,
    peerOpts: {

    }
  });

  const awareness = provider.awareness;
  const color = pickColor(userId);
  awareness.setLocalStateField("user", {
    name: name ?? "Anonymous",
    color,
    token: access.jwt,
    id: userId,
  });

  // Optional: validate peers' JWTs when their state arrives; mark invalid
  const invalidPeers = new Set<number>();
  (awareness as any).__invalidPeers = invalidPeers;
  if (jwks) {
    awareness.on("update", async ({ added, updated }: { added: number[]; updated: number[] }) => {
      const states = awareness.getStates() as Map<number, any>;
      const toCheck = [...added, ...updated];
      for (const clientId of toCheck) {
        const state = states.get(clientId);
        const token = state?.user?.token as string | undefined;
        if (!token) continue;
        const ok = await verifyPeerJwt(token, jwks);
        if (!ok) invalidPeers.add(clientId);
      }
    });
  }

  const dispose = () => {
    try { provider.destroy(); } catch {}
    try { doc.destroy(); } catch {}
  };

  return { doc, provider, awareness, dispose };
}
