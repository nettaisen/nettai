// nettai's signaling server: where two players meet in a room and trade what
// their WebRTC connection needs (the offer, the answer, the candidates),
// before it is up and whenever it is made again. A Cloudflare Worker, one
// Durable Object a room; it reads none of what the players trade. The
// protocol is nettai-rtc's `signal` module (and docs/design/rollback.md
// §4.8), version 1:
//
// - A room's code is the server's to pick, never a client's: six characters
//   of ABCDEFGHJKMNPQRSTUVWXYZ23456789 (no 0 or O, no 1, I or L: none reads
//   as another).
// - To make a room, a client opens a WebSocket to /rooms?peer=ID&v=1: ID the
//   client's own (1 to 64 of A-Z a-z 0-9 _ -), the same on every
//   reconnection. The server makes a room under a code no room has, the
//   client hosts it, and the welcome says the code.
// - To join a room, or to come back to one, a client opens a WebSocket to
//   /rooms/CODE?peer=ID&v=1 (the code in either case). A code no room has is
//   refused, and no room is made of it.
// - The room has two places: its maker hosts, the next ID to come joins; an
//   ID that comes again gets its place back, and a third is refused.
// - The server says
//   {"type":"welcome","role":"host"|"join","peer":BOOL,"code":CODE} (the
//   client's place, whether the other is there, the room's code), then
//   {"type":"peer","present":BOOL} whenever the other comes or goes.
// - {"type":"signal","data":...} from one client goes to the other as it is
//   (dropped if the other isn't there); {"type":"leave"} gives the
//   client's place up.
// - A refusal is {"type":"error","reason":REASON,"message":TEXT}, and the
//   socket closes. REASON is "not-found" (no room has that code), "full"
//   (the room has two players), "version" (the client speaks another
//   version of this protocol), "bad-request" (no peer ID, or a bad one) or
//   "busy" (no free code was found); TEXT is for a log.
// - "ping" is answered "pong" without waking the room (the keepalive).
//
// A room is kept while anyone is in it, and ten minutes after the last
// socket goes (a player whose connection dropped comes back to its place);
// then it is forgotten, and its code with it. A room both players left is
// forgotten at once.

import { DurableObject } from "cloudflare:workers";

export interface Env {
	ROOMS: DurableObjectNamespace<Room>;
}

type Role = "host" | "join";
type Places = Partial<Record<Role, string>>;
type Reason = "not-found" | "full" | "version" | "bad-request" | "busy";

/** What a room keeps of each socket (it survives the room's hibernation). */
interface Attachment {
	role: Role;
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
const PEER = /^[A-Za-z0-9_-]{1,64}$/;
const FORGET_AFTER_MS = 10 * 60 * 1000;
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
		const path = url.pathname.match(/^\/rooms(?:\/([^/]*))?\/?$/);
		if (!path) {
			return new Response("a new room is /rooms?peer=ID&v=1, a room /rooms/CODE?peer=ID&v=1\n", { status: 404 });
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
		const ask = (what: "make" | "enter", code: string) => {
			const to = new URL(`https://room/${what}`);
			to.searchParams.set("code", code);
			to.searchParams.set("peer", peer);
			return env.ROOMS.get(env.ROOMS.idFromName(code)).fetch(new Request(to, request));
		};
		if (path[1] === undefined || path[1] === "") {
			for (let i = 0; i < CODE_TRIES; i++) {
				const made = await ask("make", newCode());
				if (made.status !== 409) {
					return made;
				}
			}
			return refuse("busy", "no free code was found");
		}
		const code = path[1].toUpperCase();
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

	/** The Worker's two questions: /make (a new room under this code: 409 if
	 * it has one) and /enter (a place in the room this code has). */
	async fetch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		const peer = url.searchParams.get("peer") ?? "";
		const code = url.searchParams.get("code") ?? "";
		// (A room there is has its places stored, even when both are free.)
		let places = await this.ctx.storage.get<Places>("places");
		if (url.pathname === "/make") {
			if (places !== undefined) {
				return new Response(null, { status: 409 });
			}
			places = {};
		} else if (places === undefined) {
			return refuse("not-found", "no room has that code");
		}
		const roles: Role[] = ["host", "join"];
		const role = roles.find((r) => places[r] === peer) ?? roles.find((r) => places[r] === undefined);
		if (role === undefined) {
			return refuse("full", "the room is full");
		}
		const [client, server] = Object.values(new WebSocketPair());
		places[role] = peer;
		await this.ctx.storage.put("places", places);
		await this.ctx.storage.deleteAlarm();
		// The client's socket from before, if it is still here, goes.
		for (const old of this.ctx.getWebSockets(role)) {
			old.serializeAttachment({ role, replaced: true } satisfies Attachment);
			old.close(4001, "replaced by a new connection");
		}
		this.ctx.acceptWebSocket(server, [role]);
		server.serializeAttachment({ role } satisfies Attachment);
		const there = this.ctx.getWebSockets(other(role));
		server.send(JSON.stringify({ type: "welcome", role, peer: there.length > 0, code }));
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
			const places: Places = (await this.ctx.storage.get<Places>("places")) ?? {};
			delete places[role];
			ws.close(1000, "left");
			await this.gone(ws);
			if (places.host === undefined && places.join === undefined) {
				// Both left: the room is over, and its code free.
				await this.forget();
			} else {
				await this.ctx.storage.put("places", places);
			}
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
	 * back on another), and an empty room is forgotten in a while. */
	async gone(ws: WebSocket): Promise<void> {
		const { role, replaced } = ws.deserializeAttachment() as Attachment;
		ws.serializeAttachment({ role, replaced: true } satisfies Attachment);
		if (!replaced) {
			for (const them of this.ctx.getWebSockets(other(role))) {
				them.send(JSON.stringify({ type: "peer", present: false }));
			}
		}
		const open = this.ctx.getWebSockets().filter((s) => s !== ws && s.readyState === WebSocket.OPEN);
		if (open.length === 0) {
			await this.ctx.storage.setAlarm(Date.now() + FORGET_AFTER_MS);
		}
	}

	/** The room is over: nothing is kept of it, and its code names no room. */
	async forget(): Promise<void> {
		await this.ctx.storage.deleteAlarm();
		await this.ctx.storage.deleteAll();
	}

	async alarm(): Promise<void> {
		if (this.ctx.getWebSockets().every((s) => s.readyState !== WebSocket.OPEN)) {
			await this.forget();
		}
	}
}
