// nettai's signaling server: where two players meet in a room and trade what
// their WebRTC connection needs (the offer, the answer, the candidates),
// until it is up and whenever it is made again. A Cloudflare Worker, one
// Durable Object a room; it reads none of what the players trade. The
// protocol is nettai-rtc's `signal` module (and docs/design/rollback.md
// §4.8), version 1.
//
// The two are in a room only while they need the server: until their first
// connection is through, and while a dropped one is made again. Between
// those times neither has a socket here. There are two kinds of room:
//
// - An invite room is where two players first meet. Its code is the
//   server's to pick, never a client's: six characters of
//   ABCDEFGHJKMNPQRSTUVWXYZ23456789 (no 0 or O, no 1, I or L: none reads as
//   another). To make one, a client opens a WebSocket to /rooms?peer=ID&v=1:
//   ID the client's own (1 to 64 of A-Z a-z 0-9 _ -), the same on every
//   reconnection. The server makes a room under a code no room has, the
//   client hosts it, and the welcome says the code. To join one, or to come
//   back to one, a client opens /rooms/CODE?peer=ID&v=1. A code no room has
//   is refused, and no room is made of it.
// - A reconnect room is where the two meet again after their connection
//   dropped, by a code they agreed on over that connection (32 lowercase
//   hex digits: 128 random bits), so this server first sees it when it is
//   used. Both open /reconnect/CODE?peer=ID&v=1: the first to come makes the
//   room, the second joins it. A code not of that form is refused.
// - A room has two places. In an invite room its maker hosts and the next ID
//   to come joins; in a reconnect room the places are only the first and the
//   second to come (the clients keep the roles they had). An ID that comes
//   again gets its place back, and a third is refused.
// - The server says
//   {"type":"welcome","role":"host"|"join","peer":BOOL,"code":CODE} in an
//   invite room and {"type":"welcome","peer":BOOL} in a reconnect room (the
//   client's place, whether the other is there, the invite's code), then
//   {"type":"peer","present":BOOL} whenever the other comes or goes.
// - Once both players are in the room, a client is given "iceServers":[...]:
//   the STUN and TURN servers its connections use, exactly those (as the
//   browser's RTCIceServer: {"urls":[...],"username":...,"credential":...}),
//   in its welcome if the other is in the room already, or else in the peer
//   message as the other comes. The user's words: "the
//   signaling server should return stun and turn server credentials". With
//   its Cloudflare TURN key (the secrets CLOUDFLARE_TURN_KEY_ID and
//   CLOUDFLARE_TURN_KEY_API_TOKEN), the server asks Cloudflare's TURN
//   service for credentials for this client, which last TURN_TTL_SECONDS,
//   and gives what it answers: its STUN server and its TURN servers. Without
//   the key, or when the service fails or doesn't answer within
//   TURN_API_WAIT_MS, Cloudflare's public STUN server alone (STUN), and the
//   failure is logged here, never the secrets; a room never waits on it
//   longer.
// - A client in mainland China (by Cloudflare's lookup of its IP address,
//   country CN; Hong Kong, Macau and Taiwan have codes of their own) is
//   given the China servers instead, and only those: the owner's own STUN
//   and TURN server there, a coturn (the secrets CHINA_TURN_URLS and
//   CHINA_TURN_SECRET), with credentials the server makes itself, which
//   also last TURN_TTL_SECONDS. Without those secrets, Cloudflare's, as any
//   other client is given. The two players of a room may be given different
//   servers: each relays through its own, and the two relays reach each
//   other.
// - TURN credentials cost the key's owner what is relayed on them, so they
//   go only to a room with both its places taken (a room one client made
//   alone is no match, and a socket opened to collect credentials gets
//   none): a welcome before then gives no servers (a client makes no
//   connection before the other comes), and the client that waited is
//   given its servers when the other comes, in that peer message
//   ({"type":"peer","present":true,"iceServers":[...]}). And a client gets
//   at most TURN_PER_MINUTE of them a minute (TURN_LIMIT, a rate limit
//   binding, by its IP address, an IPv6 one's /64), past which STUN alone,
//   logged.
// - {"type":"signal","data":...} from one client goes to the other as it is
//   (dropped if the other isn't there); {"type":"leave"} gives the
//   client's place up.
// - A refusal is {"type":"error","reason":REASON,"message":TEXT}, and the
//   socket closes. REASON is "not-found" (no room has that code), "full"
//   (the room has two players), "version" (the client speaks another
//   version of this protocol), "bad-request" (no peer ID or a bad one, or a
//   reconnect code not of the clients' form) or "busy" (no free code was
//   found); TEXT is for a log.
// - "ping" is answered "pong" without waking the room (the keepalive).
//
// A room both players left is forgotten at once, and its code with it. A
// room is otherwise kept while anyone is in it, and after the last socket
// goes (a player whose socket dropped comes back to its place) ten minutes
// for an invite room, thirty seconds (a link's time to reconnect) for a
// reconnect room; then it is forgotten.

import { DurableObject } from "cloudflare:workers";

export interface Env {
	ROOMS: DurableObjectNamespace<Room>;
	/** The Cloudflare Realtime TURN key's ID, a secret (`npx wrangler secret
	 * put CLOUDFLARE_TURN_KEY_ID`): none, no Cloudflare TURN. */
	CLOUDFLARE_TURN_KEY_ID?: string;
	/** The Cloudflare TURN key's API token, a secret (`npx wrangler secret
	 * put CLOUDFLARE_TURN_KEY_API_TOKEN`). */
	CLOUDFLARE_TURN_KEY_API_TOKEN?: string;
	/** The China servers' URLs, separated by spaces or commas: the `stun:`
	 * and `turn:` URLs of the owner's coturn in mainland China, a secret
	 * (`npx wrangler secret put CHINA_TURN_URLS`). None, and clients in
	 * China are given Cloudflare's. */
	CHINA_TURN_URLS?: string;
	/** The coturn's shared secret, its `static-auth-secret`, a secret (`npx
	 * wrangler secret put CHINA_TURN_SECRET`). */
	CHINA_TURN_SECRET?: string;
	/** How often a client may be given TURN credentials (a rate limit
	 * binding, wrangler.toml's TURN_PER_MINUTE a minute): none, no limit. */
	TURN_LIMIT?: RateLimit;
}

/** A STUN or TURN server, as the browser's RTCIceServer (and nettai-rtc's
 * IceServer) has it. */
interface IceServer {
	urls: string[];
	username?: string;
	credential?: string;
}

type Role = "host" | "join";
type Places = Partial<Record<Role, string>>;
type Reason = "not-found" | "full" | "version" | "bad-request" | "busy";
/** An invite room, or a reconnect room. */
type Kind = "invite" | "reconnect";

/** What a room keeps of each socket (it survives the room's hibernation). */
interface Attachment {
	/** Its place (in a reconnect room, "host" is the first to come). */
	role: Role;
	kind: Kind;
	/** The client's country (`countryOf`), which picks its servers when
	 * they are given as the other comes. */
	country: string;
	/** Another socket of the same client took its place: the other client
	 * isn't told it went. */
	replaced?: boolean;
}

/** The protocol's version: a client that says another is refused. */
const VERSION = "1";
/** A code's characters: capitals and digits, without 0, O, 1, I and L. */
const ALPHABET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LENGTH = 6;
const CODE = /^[A-HJKMNP-Z2-9]{6}$/;
const RECONNECT_CODE = /^[0-9a-f]{32}$/;
const PEER = /^[A-Za-z0-9_-]{1,64}$/;
/** How long a room with no socket is kept, by kind. */
const FORGET_AFTER_MS: Record<Kind, number> = { invite: 10 * 60 * 1000, reconnect: 30 * 1000 };
/** How many codes a new room tries before the server says it is busy (a
 * code in use is one in some 887 million a room). */
const CODE_TRIES = 8;

/** Cloudflare's public STUN server: what a client is given when there are no
 * TURN credentials. */
const STUN: IceServer[] = [{ urls: ["stun:stun.cloudflare.com:3478"] }];
/** How long a welcome's TURN credentials last: a long sitting of matches
 * with one opponent and its reconnections (nettai-rtc's
 * signal::TURN_CREDENTIALS_LAST). */
const TURN_TTL_SECONDS = 4 * 60 * 60;
/** How long a welcome waits on Cloudflare's TURN service before it gives
 * STUN alone. */
const TURN_API_WAIT_MS = 3000;
/** Mainland China's country code, whose clients are given the China
 * servers. */
const CHINA = "CN";

/** The STUN and TURN servers for a client of a room both players are in
 * (the comment at the top): for a client in `country` China, the China
 * servers when the owner has set them; else Cloudflare's TURN service's,
 * with credentials made for it; or STUN alone. `client` is who the limit
 * counts it against (`clientOf`). Never fails, and never says a secret: a
 * failure's log is its kind or its HTTP status. */
async function iceServers(env: Env, client: string, country: string): Promise<IceServer[]> {
	const china = country === CHINA && env.CHINA_TURN_URLS && env.CHINA_TURN_SECRET ? { urls: env.CHINA_TURN_URLS, secret: env.CHINA_TURN_SECRET } : undefined;
	if (!china && (!env.CLOUDFLARE_TURN_KEY_ID || !env.CLOUDFLARE_TURN_KEY_API_TOKEN)) {
		return STUN;
	}
	try {
		if (env.TURN_LIMIT && !(await env.TURN_LIMIT.limit({ key: client })).success) {
			console.error("no TURN credentials: a client asked for more than its limit");
			return STUN;
		}
	} catch (e) {
		console.error(`no TURN credentials: ${e instanceof Error ? e.name : "an error"} asking the TURN limit`);
		return STUN;
	}
	return china ? chinaServers(china.urls, china.secret) : cloudflareServers(env.CLOUDFLARE_TURN_KEY_ID!, env.CLOUDFLARE_TURN_KEY_API_TOKEN!);
}

/** The China servers (`CHINA_TURN_URLS`), all with one username and
 * credential made here by coturn's TURN REST scheme (its `use-auth-secret`),
 * with no call to it: the username is when they expire (Unix seconds,
 * TURN_TTL_SECONDS from now) and nothing more, since a client on the web
 * can't be given a username with a `:` in it (nettai-rtc's `ice_urls`), and
 * the credential is base64(HMAC-SHA1(secret, username)). A `stun:` URL
 * among them needs none, and the clients give it none. Never fails: STUN
 * alone, logged, when the URLs are empty or the credential can't be made. */
async function chinaServers(urls: string, secret: string): Promise<IceServer[]> {
	const list = urls.split(/[\s,]+/).filter((u) => u !== "");
	if (list.length === 0) {
		console.error("no TURN credentials: CHINA_TURN_URLS names no server");
		return STUN;
	}
	try {
		const username = String(Math.floor(Date.now() / 1000) + TURN_TTL_SECONDS);
		const utf8 = new TextEncoder();
		const key = await crypto.subtle.importKey("raw", utf8.encode(secret), { name: "HMAC", hash: "SHA-1" }, false, ["sign"]);
		const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, utf8.encode(username)));
		return [{ urls: list, username, credential: btoa(String.fromCharCode(...mac)) }];
	} catch (e) {
		console.error(`no TURN credentials: ${e instanceof Error ? e.name : "an error"} making the China server's`);
		return STUN;
	}
}

/** Cloudflare's TURN service's STUN and TURN servers, with credentials it
 * made for this client, asked for with the owner's TURN key. Never fails:
 * STUN alone, logged, when the service fails or doesn't answer in time. */
async function cloudflareServers(keyId: string, apiToken: string): Promise<IceServer[]> {
	try {
		const url = `https://rtc.live.cloudflare.com/v1/turn/keys/${encodeURIComponent(keyId)}/credentials/generate-ice-servers`;
		const response = await fetch(url, {
			method: "POST",
			headers: { Authorization: `Bearer ${apiToken}`, "Content-Type": "application/json" },
			body: JSON.stringify({ ttl: TURN_TTL_SECONDS }),
			signal: AbortSignal.timeout(TURN_API_WAIT_MS),
		});
		if (!response.ok) {
			console.error(`no TURN credentials: Cloudflare's TURN service answered ${response.status}`);
			return STUN;
		}
		const servers = serversOf(await response.json());
		if (servers.length === 0) {
			console.error("no TURN credentials: Cloudflare's TURN service gave no servers");
			return STUN;
		}
		return servers;
	} catch (e) {
		console.error(`no TURN credentials: ${e instanceof Error ? e.name : "an error"} asking Cloudflare's TURN service`);
		return STUN;
	}
}

/** The servers of Cloudflare's answer ({"iceServers":[...]}), each as an
 * IceServer: its URLs a list (a string is one), and those that aren't
 * servers left out. */
function serversOf(answer: unknown): IceServer[] {
	const list = (answer as { iceServers?: unknown } | null)?.iceServers;
	if (!Array.isArray(list)) {
		return [];
	}
	const servers: IceServer[] = [];
	for (const s of list as { urls?: unknown; username?: unknown; credential?: unknown }[]) {
		const urls = (typeof s?.urls === "string" ? [s.urls] : Array.isArray(s?.urls) ? s.urls : []).filter((u): u is string => typeof u === "string");
		if (urls.length === 0) {
			continue;
		}
		const server: IceServer = { urls };
		if (typeof s.username === "string" && typeof s.credential === "string") {
			server.username = s.username;
			server.credential = s.credential;
		}
		servers.push(server);
	}
	return servers;
}

/** Who a request is, for the TURN limit: its IP address (Cloudflare's
 * CF-Connecting-IP), an IPv6 one's first 64 bits (a home network has a /64
 * of its own, and all of it is one client). */
function clientOf(request: Request): string {
	const ip = request.headers.get("CF-Connecting-IP") ?? "";
	if (!ip.includes(":") || ip.includes(".")) {
		return ip;
	}
	const [head, tail] = ip.split("::");
	const front = head ? head.split(":") : [];
	const back = tail ? tail.split(":") : [];
	const groups = tail === undefined ? front : [...front, ...Array(Math.max(0, 8 - front.length - back.length)).fill("0"), ...back];
	return `${groups.slice(0, 4).map((g) => parseInt(g, 16).toString(16)).join(":")}::/64`;
}

/** Where a request comes from, by Cloudflare's lookup of its IP address:
 * the country's ISO 3166 code, or "" when Cloudflare doesn't say. */
function countryOf(request: Request): string {
	const country = (request.cf as { country?: unknown } | undefined)?.country;
	return typeof country === "string" ? country : "";
}

/** The WebSocket close code of each refusal. */
const CLOSE: Record<Reason, number> = { full: 4000, "version": 4002, "bad-request": 4003, "not-found": 4004, busy: 4005 };

function other(role: Role): Role {
	return role === "host" ? "join" : "host";
}

/** A code drawn evenly from the alphabet. */
function newCode(): string {
	let code = "";
	// (A byte under 248, eight times the alphabet's 31, is even over it.)
	while (code.length < CODE_LENGTH) {
		for (const b of crypto.getRandomValues(new Uint8Array(2 * CODE_LENGTH))) {
			if (b < 248 && code.length < CODE_LENGTH) {
				code += ALPHABET[b % ALPHABET.length];
			}
		}
	}
	return code;
}

/** Say `text` on another client's socket, unless it has closed since (its
 * close is on its way to the room): what it misses no longer matters to it,
 * and the room's own work goes on. */
function tell(ws: WebSocket, text: string): void {
	try {
		ws.send(text);
	} catch {
		// (Closed already.)
	}
}

/** Refuse a client on a WebSocket of its own, which says why and closes (a
 * browser's WebSocket is told nothing of an HTTP refusal). */
function refuse(reason: Reason, message: string): Response {
	const [client, server] = Object.values(new WebSocketPair());
	server.accept();
	server.send(JSON.stringify({ type: "error", reason, message }));
	server.close(CLOSE[reason], message);
	return new Response(null, { status: 101, webSocket: client });
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const url = new URL(request.url);
		const invite = url.pathname.match(/^\/rooms(?:\/([^/]*))?\/?$/);
		const reconnect = url.pathname.match(/^\/reconnect\/([^/]*)\/?$/);
		if (!invite && !reconnect) {
			return new Response("a new room is /rooms?peer=ID&v=1, a room /rooms/CODE?peer=ID&v=1, a reconnect room /reconnect/CODE?peer=ID&v=1\n", { status: 404 });
		}
		if (request.headers.get("Upgrade") !== "websocket") {
			return new Response("a room is a WebSocket\n", { status: 426 });
		}
		if (url.searchParams.get("v") !== VERSION) {
			return refuse("version", `this server speaks version ${VERSION} of the signaling protocol`);
		}
		const peer = url.searchParams.get("peer") ?? "";
		if (!PEER.test(peer)) {
			return refuse("bad-request", "a peer's ID is 1 to 64 letters, digits, _ and -");
		}
		// What a room is asked is the Worker's to say, never the client's.
		// (The two kinds' rooms are apart: a reconnect room's object is named
		// by its kind as well as its code.)
		const ask = (what: "make" | "enter" | "reconnect", code: string) => {
			const to = new URL(`https://room/${what}`);
			to.searchParams.set("code", code);
			to.searchParams.set("peer", peer);
			to.searchParams.set("client", clientOf(request));
			to.searchParams.set("country", countryOf(request));
			const name = what === "reconnect" ? `reconnect/${code}` : code;
			return env.ROOMS.get(env.ROOMS.idFromName(name)).fetch(new Request(to, request));
		};
		if (reconnect) {
			if (!RECONNECT_CODE.test(reconnect[1])) {
				return refuse("bad-request", "a reconnect code is 32 lowercase hex digits");
			}
			return ask("reconnect", reconnect[1]);
		}
		if (invite![1] === undefined || invite![1] === "") {
			for (let i = 0; i < CODE_TRIES; i++) {
				const made = await ask("make", newCode());
				if (made.status !== 409) {
					return made;
				}
			}
			return refuse("busy", "no free code was found");
		}
		const code = invite![1].toUpperCase();
		if (!CODE.test(code)) {
			return refuse("not-found", "no room has that code");
		}
		return ask("enter", code);
	},
} satisfies ExportedHandler<Env>;

export class Room extends DurableObject<Env> {
	constructor(ctx: DurableObjectState, env: Env) {
		super(ctx, env);
		this.ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
	}

	/** The Worker's three questions: /make (a new invite room under this
	 * code: 409 if it has one), /enter (a place in the invite room this code
	 * has) and /reconnect (a place in the reconnect room of this code, made
	 * for the first to come). */
	async fetch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		const peer = url.searchParams.get("peer") ?? "";
		const code = url.searchParams.get("code") ?? "";
		const client = url.searchParams.get("client") ?? "";
		const country = url.searchParams.get("country") ?? "";
		const kind: Kind = url.pathname === "/reconnect" ? "reconnect" : "invite";
		// The client's place: the places read and written with nothing else
		// between (the two come to a reconnect room at about the same
		// moment, and must not take one place). A room there is has its
		// places stored, even when both are free.
		const taken = await this.ctx.blockConcurrencyWhile(async (): Promise<{ role: Role; both: boolean } | Response> => {
			let places = await this.ctx.storage.get<Places>("places");
			if (url.pathname === "/make") {
				if (places !== undefined) {
					return new Response(null, { status: 409 });
				}
				places = {};
			} else if (places === undefined) {
				if (kind === "invite") {
					return refuse("not-found", "no room has that code");
				}
				places = {};
			}
			const roles: Role[] = ["host", "join"];
			const role = roles.find((r) => places[r] === peer) ?? roles.find((r) => places[r] === undefined);
			if (role === undefined) {
				return refuse("full", "the room is full");
			}
			places[role] = peer;
			await this.ctx.storage.put("places", places);
			await this.ctx.storage.deleteAlarm();
			return { role, both: places.host !== undefined && places.join !== undefined };
		});
		if (taken instanceof Response) {
			return taken;
		}
		const { role, both } = taken;
		// The servers (never failing, nor waiting long): none until both
		// players are in the room, then for this client and for the other's
		// socket (which waited with none, or comes back to it), each by its
		// own country.
		const waiting = both ? this.ctx.getWebSockets(other(role)).map((ws) => ws.deserializeAttachment() as Attachment).find((a) => !a.replaced) : undefined;
		const [ice, theirs] = both ? await Promise.all([iceServers(this.env, client, country), waiting ? iceServers(this.env, client, waiting.country) : undefined]) : [undefined, undefined];
		const [socket, server] = Object.values(new WebSocketPair());
		// The client's socket from before, if it is still here, goes.
		for (const old of this.ctx.getWebSockets(role)) {
			old.serializeAttachment({ ...(old.deserializeAttachment() as Attachment), replaced: true } satisfies Attachment);
			old.close(4001, "replaced by a new connection");
		}
		this.ctx.acceptWebSocket(server, [role]);
		server.serializeAttachment({ role, kind, country } satisfies Attachment);
		const there = this.ctx.getWebSockets(other(role));
		// (JSON leaves out an `iceServers` that is undefined: no servers yet.)
		const welcome =
			kind === "invite" ? { type: "welcome", role, peer: there.length > 0, code, iceServers: ice } : { type: "welcome", peer: there.length > 0, iceServers: ice };
		server.send(JSON.stringify(welcome));
		const present = { type: "peer", present: true, iceServers: theirs };
		for (const ws of there) {
			tell(ws, JSON.stringify(present));
		}
		return new Response(null, { status: 101, webSocket: socket });
	}

	async webSocketMessage(ws: WebSocket, message: string | ArrayBuffer): Promise<void> {
		if (typeof message !== "string") {
			return;
		}
		const { role } = ws.deserializeAttachment() as Attachment;
		let said: { type?: string; data?: unknown };
		try {
			said = JSON.parse(message);
		} catch {
			return;
		}
		if (said.type === "signal") {
			const relayed = JSON.stringify({ type: "signal", data: said.data });
			for (const them of this.ctx.getWebSockets(other(role))) {
				tell(them, relayed);
			}
		} else if (said.type === "leave") {
			// (The places read and written with nothing else between: the two
			// leave at about the same moment once their connection is through,
			// and each must see the other's leaving.)
			await this.ctx.blockConcurrencyWhile(async () => {
				const places: Places = (await this.ctx.storage.get<Places>("places")) ?? {};
				delete places[role];
				if (places.host === undefined && places.join === undefined) {
					// Both left: the room is over, and its code free.
					await this.forget();
				} else {
					await this.ctx.storage.put("places", places);
				}
			});
			ws.close(1000, "left");
			await this.gone(ws);
		}
	}

	async webSocketClose(ws: WebSocket, code: number, reason: string): Promise<void> {
		try {
			ws.close(code, reason);
		} catch {
			// (Closed already.)
		}
		await this.gone(ws);
	}

	async webSocketError(ws: WebSocket): Promise<void> {
		await this.gone(ws);
	}

	/** A socket went: the other client hears it (unless the same client is
	 * back on another), and a room with no socket left is forgotten in a
	 * while (its kind's; one forgotten already, both having left, is
	 * nothing to wait for). */
	async gone(ws: WebSocket): Promise<void> {
		const attachment = ws.deserializeAttachment() as Attachment;
		const { role, kind, replaced } = attachment;
		ws.serializeAttachment({ ...attachment, replaced: true } satisfies Attachment);
		if (!replaced) {
			for (const them of this.ctx.getWebSockets(other(role))) {
				tell(them, JSON.stringify({ type: "peer", present: false }));
			}
		}
		const open = this.ctx.getWebSockets().filter((s) => s !== ws && s.readyState === WebSocket.OPEN);
		if (open.length === 0 && (await this.ctx.storage.get<Places>("places")) !== undefined) {
			await this.ctx.storage.setAlarm(Date.now() + FORGET_AFTER_MS[kind]);
		}
	}

	/** The room is over: nothing is kept of it, and its code names no room.
	 * (The places are all a room stores.) */
	async forget(): Promise<void> {
		await this.ctx.storage.deleteAlarm();
		await this.ctx.storage.delete("places");
	}

	async alarm(): Promise<void> {
		if (this.ctx.getWebSockets().every((s) => s.readyState !== WebSocket.OPEN)) {
			await this.forget();
		}
	}
}
