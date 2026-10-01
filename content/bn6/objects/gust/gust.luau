--!strict
-- A gust (attack object #0x49, `sub_80CD2EC`): a wind that blows along
-- its row 10 pixels a tick, pushing what it hits, until it leaves the
-- field, reaches a panel it can't cross, is hit by a wind stopper, or (as
-- some spawners ask) hits anything. WindRack blows unseen ones down each
-- row; Wind and Fan blow one row at a time; TenguCross's B+Back blows
-- three (instant chip effect 0x14).
--
-- How it blows (`GustHow`, the original's first parameter): `pulls` (bit
-- 0) turns it back toward its side, a weaker push (hit modifier 4 rather
-- than 8) in its other palette; `unseen` (bit 7) hides it and lets it
-- blow through any panel.

local collision = require("../../rules/collision")

local gust = {}

-- How a gust blows.
export type GustHow = { pulls: boolean?, unseen: boolean? }

export type State = {
    -- ExtraVars+0: it ends at its first hit (spawners set it).
    stop_on_hit: boolean,
    -- ExtraVars+4: a spawner that tracks it: the gust ends once
    -- `holder.state.gusts[slot]` is 0, and sets it to 0 when it ends. (No
    -- spawner in the game does.)
    holder: Object?,
    slot: number,
    -- How it blows (the original's first parameter's bits 0 and 7).
    pulls: boolean,
    unseen: boolean,
}

local SPRITE = asset.sprite("gust")

-- Its element byte (5: past the elements, what nothing is weak to).
local ELEMENT = 5

-- Its speed, 16.16 per tick.
local SPEED = 0xA_0000

-- The push its hit gives: a gust that pulls pushes weaker.
local PUSH, WEAK_PUSH = 8, 4
-- The palette of a gust that pulls (its first parameter times 2).
local PULL_PALETTE = 2

-- A hit that stops it (the collision's hit flags).
local STOPPED = 0x800000

-- The panels it can blow over (`byte_80CD478`, by side: require, forbid):
-- the other side's panels. A side of 2 (a turned navi's side shifted by
-- its flip) reads the code after the table.
local PANELS = {
    [0] = { 0x20, 0x00 },
    [1] = { 0x00, 0x20 },
    [2] = { 0xB427B500, 0xF7352049 },
}

-- The tracking spawner's flag, and clearing it.
local function tracked(s: State): boolean
    local h = s.holder
    return h == nil or (h.state :: any).gusts[s.slot + 1] ~= 0
end

local function untrack(s: State)
    local h = s.holder
    if h then
        (h.state :: any).gusts[s.slot + 1] = 0
    end
end

-- The end: no region, hidden, destroyed next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CD414`: blow on; end on a stopping hit, off the field, or (unless
-- unseen) on a panel it can't blow over.
local function blow(me: Object)
    local s = me.state :: State
    local c = me.collision
    local ends = bit32.btest(c.hit_flags, STOPPED)
    if not ends then
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
        me:set_panel_from_coordinates()
        me:set_collision_panel()
        local x, y = me.panel_x, me.panel_y
        if not field.valid(x, y) then
            ends = true
        elseif s.unseen then
            return
        else
            local rule = PANELS[me.alliance]
            if rule == nil then
                error(string.format("gust side %d reads past byte_80CD478 (sub_80CD414)", me.alliance))
            end
            ends = not field.check(x, y, rule[1], rule[2])
        end
    end
    if ends then
        c:set_region(nil)
        untrack(s)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80CD3B0`: resolve its hits, then blow on (and register again).
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    if s.stop_on_hit and c.hit_flags ~= 0 then
        return finish(me)
    end
    if not tracked(s) then
        return finish(me)
    end
    if battle.over() then
        untrack(s)
        return finish(me)
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("gust action %#x reads past its table (sub_80CD3B0)", a))
    end
    blow(me)
    c:present()
end

-- `sub_80CD310`: its look, its speed along the row (the way a pull turns
-- it), its registration.
local function init(me: Object)
    local s = me.state :: State
    if not s.unseen then
        me.visible = true
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:step()
    local turn = if s.pulls then 1 else 0
    sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), turn))
    me.element = ELEMENT
    sprite.palette = if s.pulls then PULL_PALETTE else 0
    me:set_coordinates_from_panel()
    local p = me.pos
    -- object_getFlipDirection(side ^ turn, flip)
    local dir = 1 - 2 * bit32.bxor(bit32.bxor(me.alliance, turn), me.flip)
    me.pos = Vec3.new(p.x, p.y, 0)
    me.vel = Vec3.new(int.mul32(dir, SPEED), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.pull, collision.hits_navis, if s.pulls then WEAK_PUSH else PUSH)
    me.collision.self_damage = 0
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

gust.kind = define.kind {
    id = "gust",
    pool = "attack",
    state = { stop_on_hit = "bool", holder = "object", slot = "u8", pulls = "bool", unseen = "bool" },
    -- `sub_80CD2EC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- `sub_80CD488`: a gust on (x, y) for side `alliance` and flip `flip`
-- (the spawner's halfword), blowing as `how` says, with the damage word
-- `damage`, and the spawner that tracks it by `holder.state.gusts[slot]`
-- (which becomes 1), if any. Its position is the spawner's registers,
-- which its init overwrites.
function gust.spawn(
    x: number,
    y: number,
    alliance: number,
    flip: number,
    how: GustHow,
    damage: number,
    holder: Object?,
    slot: number
): Object?
    local o = battle.spawn(gust.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = alliance, flip
    o:set_damage_word(damage)
    local s = o.state :: State
    s.holder = holder
    s.slot = slot
    s.pulls = how.pulls == true
    s.unseen = how.unseen == true
    if holder then
        (holder.state :: any).gusts[slot + 1] = 1
    end
    return o
end

return gust
