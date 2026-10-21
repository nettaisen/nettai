--!strict
-- EXE5 DS's shout (docs/design/exe5ds-map.md §4.2's microphone): a player whose navi is low on HP shouts into the
-- microphone, and once a battle the shout heals it, charges its buster or raises a barrier. The DS's own code (overlay
-- 423's 0x022261A4 and its effect object T4 0x93, 0x02225FB0; no GBA counterpart). The engine carries the microphone's
-- level in each tick's input (`battle.mic_level`), and asks the rules each fight tick a side's isn't 0 (`mic_heard`,
-- where the DS's battle runs 0x022261A4: after the supports, before the navis copy their input). Nothing in nettai sets
-- the level, so in every match and replay this never runs. (A DS recording of a netbattle shows the level reaching the
-- input record and no heal: which of the gate's conditions held it back there is open.)
--
-- The trigger (0x022261A4), when the gate passes (0x02226278: the navi on the field not in Full Synchro; MegaMan, if he
-- fights, worried, EXE5's emotion 1; its HP a quarter of its maximum or less; the side's shout not spent this battle)
-- and the battle is neither dimmed, over nor paused, and the level is 30 or more: the battle dims, the shout is spent
-- (the side's battle flag 0x800 or 0x1000), and the heal comes, its kind by a draw of the shared RNG from a row by the
-- navi (a barrier on it; its buster's levels together 7 or more), its strength by the level (over 0x4B 2, over 0x28 1,
-- else 0). (The DS also sets its save's event flags 0x2E8 and 0x2E9, and offline refuses a shout in some story states:
-- the save's.)

local barriers = require("../lib/barriers")
local effects = require("../lib/effects")

local shout = {}

type State = { shouted: boolean }

-- The level a shout needs, and its strengths' lines.
local LOUD, STRONGER, STRONGEST = 0x1E, 0x28, 0x4B

-- What the heal does (the effect object's parameter 5): heal, charge the buster, raise a barrier.
local HEAL, BUSTER, BARRIER = 0, 1, 2

-- The rows (0x0222B354, sixteen kinds each) by the navi: no barrier and its buster's levels under 7; a barrier; the
-- levels 7 or more; both.
local ROWS: { { number } } = {
    { 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2 },
    { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1 },
    { 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 2, 2, 2 },
    { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1 },
}

-- By the strength: the HP healed (0x0222B310), the barrier raised (0x0222B319).
local HP: { number } = { 100, 200, 300 }
local BARRIERS: { Barrier } = { barriers.barrier_10, barriers.barrier_100, barriers.barrier_200 }
-- What the buster's charge adds: the halfword the DS reads at 0x0222B316 + the strength, an odd address its ARM9
-- reads aligned (to confirm on a console): 0x0201, 0x0201, 0x0103.
local BUSTER_ADDS: { number } = { 0x0201, 0x0201, 0x0103 }

local OK_SOUND = asset.sound("ok-8b")
local HEAL_SOUND = asset.sound("heal")
local FADE_SPEED = 4
local WAIT = 0x1E

-- 0x02226278
local function gate(side: number, navi: Object): boolean
    local stats = battle.navi(side)
    if stats.mood == 0xFF then
        return false
    end
    if stats.navi.forms ~= nil and battle.emotion(side) ~= "worried" then
        return false
    end
    if navi.hp > bit32.rshift(navi.max_hp, 2) then
        return false
    end
    return not (rules.state() :: State).shouted
end

-- The effect object's steps (its state 4's sub-states on the object's phase; its timer, `timer`).
local FADE, STRENGTH, ACT, UNDIM = 0x0, 0x4, 0x8, 0x14

local function set_phase(me: Object, phase: number)
    me.phase = phase
    me.phase_init = 0
end

type Params = { strength: number, kind: number, navi: Object }

local function params(me: Object): Params
    local s = me.state :: { strength: number, kind: number }
    return { strength = s.strength, kind = s.kind, navi = me.related1 :: Object }
end

-- A wait of `WAIT` ticks, then `next`.
local function wait(me: Object, next: number)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        set_phase(me, next)
    end
end

-- 0x02226048, 0x02226094, 0x0222610C: the heal of its kind, then 30 ticks.
local function act(me: Object, p: Params)
    if me.phase_init == 0 then
        local navi = p.navi
        if p.kind == HEAL then
            navi.hp = math.min(navi.hp + HP[p.strength + 1], navi.max_hp)
            battle.effect(navi.pos, effects.heal)
            battle.play_sound(HEAL_SOUND)
        elseif p.kind == BUSTER then
            local stats = battle.navi(navi.alliance)
            local n = BUSTER_ADDS[p.strength + 1]
            stats.attack = int.u8(stats.attack + n)
            stats.charge = math.min(stats.charge + n, 4)
            stats.rapid = math.min(stats.rapid + n, 4)
            battle.effect(Vec3.new(navi.pos.x, navi.pos.y, 0x10_0000), effects.heal, nil, 1)
            battle.play_sound(HEAL_SOUND)
        else
            barriers.raise(navi, BARRIERS[p.strength + 1])
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    wait(me, UNDIM)
end

local function update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        -- 0x02225FC0: its sound (and the DS's voice for MegaMan, a sound the GBA hasn't: left out).
        battle.play_sound(OK_SOUND)
        me:set_lifecycle("update")
        set_phase(me, FADE)
    elseif l ~= "update" then
        -- 0x0222617C
        battle.set_dimmed(false)
        me:free()
        return
    end
    local p = params(me)
    local phase = me.phase
    if phase == FADE then
        if me.phase_init == 0 then
            battle.screen_fade("dim", FADE_SPEED)
            me.phase_init = 4
        end
        if not battle.screen_fading() then
            set_phase(me, STRENGTH)
        end
    elseif phase == STRENGTH then
        -- 0x02226014: the strongest shout puts the navi in Full Synchro first (unless its mood is held) and waits.
        if me.phase_init == 0 then
            if p.strength ~= 2 then
                set_phase(me, ACT)
                return
            end
            battle.set_mood(me.alliance, 0xFF)
            me.timer = WAIT
            me.phase_init = 4
        end
        wait(me, ACT)
    elseif phase == ACT then
        act(me, p)
    elseif phase == UNDIM then
        -- 0x02226158
        if me.phase_init == 0 then
            battle.screen_fade("undim", FADE_SPEED)
            me.phase_init = 4
        end
        if not battle.screen_fading() then
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("the shout's step %#x reads past its table (0x0222B33C)", phase))
    end
end

-- T4 0x93 (0x02225FB0).
shout.kind = new.kind {
    id = "shout",
    pool = "effect",
    state = { strength = "u8", kind = "u8" },
    update = update,
}

-- 0x022261A4 (the rules' `mic_heard`).
function shout.mic_heard(side: number)
    local navi = battle.player(side)
    if navi == nil or not gate(side, navi) then
        return
    end
    if battle.dimmed() or battle.over() or battle.paused() then
        return
    end
    local level = battle.mic_level(side)
    if level < LOUD then
        return
    end
    battle.set_dimmed(true)
    local state = rules.state() :: State
    state.shouted = true
    local stats = battle.navi(side)
    local row = 1
    if navi.collision.barrier ~= "none" then
        row += 1
    end
    if stats.attack + stats.charge + stats.rapid >= 7 then
        row += 2
    end
    local kind = ROWS[row][bit32.band(bit32.rshift(battle.rng_positive(), 5), 0xF) + 1]
    local strength = if level > STRONGEST then 2 elseif level > STRONGER then 1 else 0
    local o = battle.spawn(shout.kind)
    if o then
        -- (It runs through the dimming it starts, which only its end lifts.)
        o.run_while_dimmed = true
        o.alliance = navi.alliance
        o.flip = navi.flip
        o.related1 = navi
        local s = o.state :: { strength: number, kind: number }
        s.strength, s.kind = strength, kind
    end
end

return shout
