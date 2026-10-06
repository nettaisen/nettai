// nettai's signaling server: where two players meet by a room code and
// trade what their WebRTC connection needs (the offer, the answer, the
// candidates), before it is up and whenever it is made again. A Cloudflare
// Worker, one Durable Object a room; it reads none of what the players
// trade. The protocol is nettai-rtc's `signal` module (and
// docs/design/rollback.md §4.8):
//
// - A client opens a WebSocket to /rooms/CODE?peer=ID: CODE the room's (1
//   to 64 of A-Z a-z 0-9 _ -), ID the client's own, the same on every
//   reconnection.
// - The room has two places: the first ID to come hosts, the second joins;
//   an ID that comes again gets its place back, and a third is refused
//   ({"type":"error","message":"the room is full"}).
// - The server says {"type":"welcome","role":"host"|"join","peer":BOOL}
//   (the client's place, and whether the other is there), then
//   {"type":"peer","present":BOOL} whenever the other comes or goes.
// - {"type":"signal","data":...} from one client goes to the other as it is
//   (dropped if the other isn't there); {"type":"leave"} gives the
//   client's place up.
// - "ping" is answered "pong" without waking the room (the keepalive).
//
// A room's places are kept while anyone is in it, and ten minutes after the
// last socket goes (a player whose connection dropped comes back to its
// place); then they are forgotten.

import { DurableObject } from "cloudflare:workers";

export interface Env {
	ROOMS: DurableObjectNamespace<Room>;
}

type Role = "host" | "join";
type Places = Partial<Record<Role, string>>;

/** What a room keeps of each socket (it survives the room's hibernation). */
interface Attachment {
	role: Role;
	/** Another socket of the same client took its place: the other client
	 * isn't told it went. */
	replaced?: boolean;
}

const NAME = /^[A-Za-z0-9_-]{1,64}$/;
const FORGET_AFTER_MS = 10 * 60 * 1000;

function other(role: Role): Role {
	return role === "host" ? "join" : "host";
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const url = new URL(request.url);
		const room = url.pathname.match(/^\/rooms\/([^/]+)$/)?.[1];
		if (!room || !NAME.test(room) || !NAME.test(url.searchParams.get("peer") ?? "")) {
			return new Response("a room is /rooms/CODE?peer=ID: 1 to 64 letters, digits, _ and - each\n", { status: 404 });
		}
		if (request.headers.get("Upgrade") !== "websocket") {
			return new Response("a room is a WebSocket\n", { status: 426 });
		}
		return env.ROOMS.get(env.ROOMS.idFromName(room)).fetch(request);
	},
} satisfies ExportedHandler<Env>;

export class Room extends DurableObject<Env> {
	constructor(ctx: DurableObjectState, env: Env) {
		super(ctx, env);
		this.ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
	}

	async fetch(request: Request): Promise<Response> {
		const peer = new URL(request.url).searchParams.get("peer") ?? "";
		const [client, server] = Object.values(new WebSocketPair());
		const places: Places = (await this.ctx.storage.get<Places>("places")) ?? {};
		const roles: Role[] = ["host", "join"];
		const role = roles.find((r) => places[r] === peer) ?? roles.find((r) => places[r] === undefined);
		if (role === undefined) {
			server.accept();
			server.send(JSON.stringify({ type: "error", message: "the room is full" }));
			server.close(4000, "the room is full");
			return new Response(null, { status: 101, webSocket: client });
		}
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
		server.send(JSON.stringify({ type: "welcome", role, peer: there.length > 0 }));
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
			await this.ctx.storage.put("places", places);
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

	async alarm(): Promise<void> {
		if (this.ctx.getWebSockets().every((s) => s.readyState !== WebSocket.OPEN)) {
			await this.ctx.storage.deleteAll();
		}
	}
}
