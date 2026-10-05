--!strict
-- EXE5's computer navis (docs/design/exe5-map.md §15.9): the navis no player controls a battle brings, which the
-- navi type's tables drive by their AI index (its idle, 0x081041F0's action 6, and its tick, 0x080F23FC's entry).
-- In a netbattle the one is Dark MegaMan (AI index 0x16, navis/dark-megaman), whom a failed Chaos Unison brings
-- (rules/souls/chaos) for twelve seconds: his idle runs the AI (./ai) until his time is up, then he leaves; his last
-- three seconds he blinks. The system drives each such navi of its side's summoning; the navi's state is the AI's
-- (./ai's State, AIData +0x80 on). It also drives a side's own player under DarkInvs (the no-charge drive, the
-- status section's `no_charge_drive`: the player's idle asks it where it would step), on the AI's other family
-- (0x0802B4AC), its state the system's of that side.

local ai = require("@self/ai")

local computer_navi = {}

-- The ticks Dark MegaMan stays (0x08104218).
local LIFETIME = 0x2D0
-- The ticks of his last blinks (0x08104252).
local BLINKING = 180

-- The leave's ticks after its first (0x081042F6).
local LEAVE_TICKS = 3

-- Dark MegaMan's AI index, the one this system answers for.
local DARK_MEGAMAN = 0x16

-- What the system reads of the AI's state (./ai's State: AIData +0x83 and +0x88).
type Lifetime = { timed: boolean, lifetime: number }

local function state(navi: Object): Lifetime
    return navi.state :: Lifetime
end

local function check(navi: Object)
    if navi.ai_index ~= DARK_MEGAMAN then
        error(string.format("the computer navis' tables have no AI index %#x for a netbattle (only Dark MegaMan's, 0x16)", navi.ai_index))
    end
end

export type LeaveState = {
    -- AIAttackVars+0x10: ticks left.
    timer: number,
}

-- 0x081042C8 (the navi type's action 7 for AI index 0x16): his leave. He drops his guard (animation 4, no collision
-- region), his attachment and his barrier's look go, his status visuals and his chips are forgotten; four ticks
-- later he leaves the battle (no HP, off the lists, destroyed).
local function leave(me: Object, s: LeaveState)
    if me.step_init == 0 then
        me:set_animation(4)
        me.collision.region = nil
        me.related1 = nil
        local visual = me.barrier_visual
        if visual then
            visual:set_lifecycle("destroy")
        end
        me:drop_links()
        s.timer = LEAVE_TICKS
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me:leave()
    end
end

computer_navi.leave = define.action {
    id = "computer-navi/leave",
    state = { timer = "u16" },
    update = leave,
}

-- 0x08104210 (AI index 0x16's idle): his twelve seconds start with his first idle; when they are up he leaves,
-- else the AI decides (0x0802BA14).
local function idle(navi: Object)
    local s = state(navi)
    if not s.timed then
        s.lifetime = LIFETIME
        s.timed = true
    end
    if s.lifetime == 0 then
        navi:set_attack(computer_navi.leave, 0)
        return
    end
    ai.run(navi, navi.state)
end

-- 0x08104234 (AI index 0x16's tick): unless the battle is paused or dimmed, his time runs down (not below 0), and in
-- his last three seconds he is hidden every other pair of ticks; when the battle is over his time is up.
local function tick(navi: Object)
    if battle.paused() or battle.dimmed() then
        return
    end
    local s = state(navi)
    s.lifetime = math.max(s.lifetime - 1, 0)
    if s.lifetime < BLINKING and bit32.band(s.lifetime, 2) ~= 0 then
        navi.visible = false
    end
    if battle.over() then
        s.lifetime = 0
    end
end

-- 0x08104284: the Dark MegaMen on the side across from `side` (its alive actors of actor type navi and AI index
-- 0x16) see their time up.
function computer_navi.dismiss(side: number)
    local other = bit32.bxor(side, 1)
    for i = 1, 4 do
        local navi = battle.alive_actor_slot(other, i)
        if navi ~= nil and navi.actor_type == "navi" and navi.ai_index == DARK_MEGAMAN then
            state(navi).lifetime = 0
        end
    end
end

computer_navi.system = define.system {
    id = "computer-navi",
    navi_state = ai.layout,
    -- A player's own drive (DarkInvs): its AI's state, AIData +0xF0 on.
    state = ai.layout,
    actions = { computer_navi.leave },
    hooks = {
        -- The navi type's idle (action 6) of a navi this system drives: its AI index's. And the side's player's
        -- idle where it would step (EXE5's no-charge drive, 0x080F03E4): with the no-charge state the AI decides
        -- for it (0x0802B4AC: DarkInvs's drive, on its own side's tactics, its state the system's of the side);
        -- without, that state starts over (0x0802C03A) and its idle goes on.
        controller = function(_side: number, navi: Object)
            if navi.actor_type == "player" then
                local s = system.state()
                if navi:navi_state("no_charge") then
                    -- A drive started since its last step (its timer above the one it stepped with): the state
                    -- from the start, as the drive's start leaves it (0x0802C03A, then AIData +0xF0 set again).
                    if navi.no_charge_timer > s.drive_ticks then
                        ai.reset(navi, s)
                        navi.computer_driven = true
                    end
                    s.drive_ticks = navi.no_charge_timer
                    ai.run_own(navi, s)
                    return "nothing"
                end
                ai.reset(navi, s)
                s.drive_ticks = 0
                return nil
            end
            check(navi)
            idle(navi)
            return "nothing"
        end,
        -- The navi type's tick of its AI index (0x080F23FC's entry), for a navi of the navi type (a player's navi
        -- has its own tick: the side's other systems' hooks).
        navi_tick = function(_side: number, navi: Object)
            if navi.actor_type ~= "navi" then
                return
            end
            check(navi)
            tick(navi)
        end,
    },
}

return computer_navi
