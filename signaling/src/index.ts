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
		const kind: Kind = url.pathname === "/reconnect" ? "reconnect" : "invite";
		// The client's place: the places read and written with nothing else
		// between (the two come to a reconnect room at about the same
		// moment, and must not take one place). A room there is has its
		// places stored, even when both are free.
		const taken = await this.ctx.blockConcurrencyWhile(async (): Promise<Role | Response> => {
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
			return role;
		});
		if (taken instanceof Response) {
			return taken;
		}
		const role = taken;
		const [client, server] = Object.values(new WebSocketPair());
		// The client's socket from before, if it is still here, goes.
		for (const old of this.ctx.getWebSockets(role)) {
			old.serializeAttachment({ role, kind, replaced: true } satisfies Attachment);
			old.close(4001, "replaced by a new connection");
		}
		this.ctx.acceptWebSocket(server, [role]);
		server.serializeAttachment({ role, kind } satisfies Attachment);
		const there = this.ctx.getWebSockets(other(role));
		const welcome = kind === "invite" ? { type: "welcome", role, peer: there.length > 0, code } : { type: "welcome", peer: there.length > 0 };
		server.send(JSON.stringify(welcome));
		for (const ws of there) {
			ws.send(JSON.stringify({ type: "peer", present: true }));
		}
		return new Response(null, { status: 101, webSocket: client });
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
				them.send(relayed);
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
		const { role, kind, replaced } = ws.deserializeAttachment() as Attachment;
		ws.serializeAttachment({ role, kind, replaced: true } satisfies Attachment);
		if (!replaced) {
			for (const them of this.ctx.getWebSockets(other(role))) {
				them.send(JSON.stringify({ type: "peer", present: false }));
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
