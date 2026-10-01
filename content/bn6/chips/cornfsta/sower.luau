--!strict
-- CornFsta's sower (the original's attack object #0xAA, `sub_80DAA28`;
-- docs/engine/dimming-chip-effects.md §16): an unseen probe that steps a
-- panel from in front of the farmer and bursts there (or where it first
-- touches a body), then sets off 15 more bursts, 8 ticks apart, around
-- its panel: on an enemy at random about three times in eight, else on a
-- free panel at random, avoiding the last three. Its bursts are CornSht's
-- corns (chips/cornsht/corn) that don't spread.

local collision = require("../../rules/collision")
local panels = require("../../lib/panels")
local corn = require("../cornsht/corn")

local sower = {}

export type State = {
    -- Its steps left (ExtraVars+0, counting down to 1).
    steps: number,
    -- The object whose `sowing` it clears when it goes (the original's
    -- pointer to the farmer's fourth parameter).
    holder: Object?,
    -- The last three bursts' panels (+0x6C, packed y << 4 | x), and where
    -- the next goes in them (ExtraVars+0x10).
    ring: { number },
    ring_next: number,
}

local STEPS = 2
-- A burst every 8 ticks, 16 in all (the first included).
local PERIOD, BURSTS = 8, 0x10
-- The camera shakes with each burst.
local SHAKE, SHAKE_TICKS = 1, 0x14
-- The panels around its own (`byte_80DACC0`), its own first.
local AROUND = { { 0, 0 }, { 0, -1 }, { 1, -1 }, { 1, 0 }, { 1, 1 }, { 0, 1 }, { -1, 1 }, { -1, 0 }, { -1, -1 } }
-- An enemy's body (`off_80DABFC`, by side), which the free panels lack
-- (`dword_80DACB0`).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
-- An attack in the enemy's direction about 6 times in 16 (the low four
-- bits of a draw).
local AT_ENEMY = 6

-- `sub_80C67D8` (with flags |= 0x10): a burst on (x, y) that doesn't
-- spread, running while dimmed; the camera shakes.
local function burst(me: Object, x: number, y: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local o = corn.spawn(me, x, y, 0, me.element, damage, corn.NO_SPREAD)
    if o then
        o.run_while_dimmed = true
    end
    battle.shake_camera(SHAKE, SHAKE_TICKS)
end

-- `sub_80DABE8` (`sub_8015D80`): of the nine around (x, y), dx toward the
-- enemy, those with an enemy's body; one at random (a `rng_positive`
-- draw), or nil.
local function enemy_near(me: Object, x: number, y: number): number?
    local dir = 1 - 2 * me.alliance
    local found: { number } = {}
    for _, d in AROUND do
        local px, py = x + d[1] * dir, y + d[2]
        if field.check(px, py, ENEMY_BODY[me.alliance], 0) then
            found[#found + 1] = panels.pack(px, py)
        end
    end
    if #found == 0 then
        return nil
    end
    return found[battle.rng_positive() % #found + 1]
end

-- `sub_80DAC0C`: of the nine around (x, y), dx as they are, those without
-- an enemy's body (and on the field); one at random (a `rng_positive`
-- draw) of those not among the last three bursts', else of all; nil if
-- none.
local function free_near(me: Object, s: State, x: number, y: number): number?
    local all: { number } = {}
    local fresh: { number } = {}
    for _, d in AROUND do
        local px, py = x + d[1], y + d[2]
        if field.check(px, py, 0, ENEMY_BODY[me.alliance]) then
            local p = panels.pack(px, py)
            all[#all + 1] = p
            if p ~= s.ring[1] and p ~= s.ring[2] and p ~= s.ring[3] then
                fresh[#fresh + 1] = p
            end
        end
    end
    local list = if #fresh > 0 then fresh else all
    if #list == 0 then
        return nil
    end
    return list[battle.rng_positive() % #list + 1]
end

-- `sub_80DABC4`: where the next burst goes (one `rng` draw first).
local function next_panel(me: Object, s: State): number?
    local p: number? = nil
    if bit32.band(battle.rng(), 0xF) < AT_ENEMY then
        p = enemy_near(me, me.panel_x, me.panel_y)
    end
    if not p then
        p = free_near(me, s, me.panel_x, me.panel_y)
    end
    return p
end

-- `sub_80DAABC`: step forward (gone off the field), and burst after the
-- last step, or where it first touches a body.
local function step(me: Object)
    local s = me.state :: State
    local c = me.collision
    if c.hit_flags == 0 then
        me.panel_x = me.panel_x + me.vel.x
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        if not field.valid(me.panel_x, me.panel_y) then
            c:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
        s.steps -= 1
        if s.steps > 1 then
            return
        end
    end
    burst(me, me.panel_x, me.panel_y)
    c:set_region(nil)
    me.action, me.phase, me.phase_init = 4, 0, 0
end

-- `sub_80DAB22`: every 8 ticks another burst near its panel, remembered;
-- after the 16th (or once the battle is over, or with no panel) gone.
local function sow(me: Object)
    local t = me.timer + 1
    me.timer = t
    if t < PERIOD then
        return
    end
    me.timer = 0
    local n = me.timer2 + 1
    me.timer2 = n
    local s = me.state :: State
    local p = if n < BURSTS and not battle.over() then next_panel(me, s) else nil
    if not p then
        me:set_lifecycle("destroy")
        return
    end
    burst(me, panels.x(p), panels.y(p))
    s.ring[s.ring_next + 1] = p
    s.ring_next = if s.ring_next + 1 >= 3 then 0 else s.ring_next + 1
end

-- `sub_80DAA86`: its action between taking its hits and presenting.
local function tick(me: Object)
    me.collision:remove()
    local a = me.action
    if a == 0 then
        step(me)
    elseif a == 4 then
        sow(me)
    else
        error(string.format("CornFsta's sower: action %#x reads past off_80DAAB4", a))
    end
    me.collision:present()
end

-- `sub_80DAA48`: on its panel, a probe that reaches bodies without
-- hurting them (no element); a panel a step; its update first runs next
-- tick. (Without a collision slot it is freed.)
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.probe, collision.hits_navis, 0)
    me.collision.element = 0
    me.collision:present()
    local s = me.state :: State
    s.ring_next = 0
    for i = 1, 3 do
        s.ring[i] = 0
    end
    s.steps = STEPS
    me:set_lifecycle("update")
end

sower.kind = define.kind {
    id = "cornfsta/sower",
    pool = "attack",
    state = { steps = "i32", holder = "object", ring = "u8[3]", ring_next = "u8" },
    -- `sub_80DAA28`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80DAB8A: the holder goes on; destroyed.
            local holder = (me.state :: State).holder
            if holder and holder.active then
                (holder.state :: { sowing: boolean }).sowing = false
            end
            me:destroy()
        end
    end,
}

-- `sub_80DAB9C`: a sower on (`x`, `y`) of `spawner`'s side and facing
-- (its first related object), with its element and the damage word
-- `damage`, running while dimmed; `holder` (an object with a boolean
-- `sowing`, set now) goes on once it is gone. (Its X and Y are the
-- spawner's registers until its init.)
function sower.spawn(spawner: Object, x: number, y: number, damage: number, holder: Object): Object?
    local o = battle.spawn(sower.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = spawner.element
    o:set_damage_word(damage)
    o.alliance, o.flip = spawner.alliance, spawner.flip
    o.related1 = spawner
    ;(o.state :: State).holder = holder
    ;(holder.state :: { sowing: boolean }).sowing = true
    o.run_while_dimmed = true
    return o
end

return sower
