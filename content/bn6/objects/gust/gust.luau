--!strict
-- A gust (attack object #0x49, `sub_80CD2EC`): a wind that blows along
-- its row 10 pixels a tick, pushing what it hits, until it leaves the
-- field, reaches a panel it can't cross, is hit by a wind stopper, or (as
-- some spawners ask) hits anything. TenguCross's B+Back blows three
-- (instant chip effect 0x14); WindRack and others blow them too.
--
-- Its first parameter: bit 0 pushes with hit modifier 4 rather than 8 (and
-- turns the sprite), bits 0..6 also turn the way it blows, bit 7 makes it
-- invisible and blow through any panel.

export type State = {
    -- ExtraVars+0: it ends at its first hit (spawners set it).
    stop_on_hit: boolean,
    -- ExtraVars+4: a spawner that tracks it: the gust ends once
    -- `holder.state.gusts[slot]` is 0, and sets it to 0 when it ends.
    holder: Object?,
    slot: number,
}

local gust = {
    state = { stop_on_hit = "bool", holder = "object", slot = "u8" },
}

local SPRITE = "0c-2e"

-- Its element byte (5: past the elements, what nothing is weak to).
local ELEMENT = 5

-- Its speed, 16.16 per tick.
local SPEED = 0xA_0000

-- What it is and what it hits (collision types), and the push its hit
-- gives by the first parameter's bit 0.
local SELF_TYPE, TARGET_TYPE = 0x1E, 5
local PUSH, WEAK_PUSH = 8, 4

-- Param1's bits.
local WEAK, INVISIBLE = 0x01, 0x80

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

-- `sub_80CD488`: a gust on (x, y) for side `alliance` and flip `flip`
-- (the spawner's halfword), with `params` (Param1..4), the damage word
-- `damage`, and the spawner that tracks it by `holder.state.gusts[slot]`
-- (which becomes 1), if any. Its position is the spawner's registers,
-- which its init overwrites.
function gust.spawn(
    x: number,
    y: number,
    alliance: number,
    flip: number,
    params: { number },
    damage: number,
    holder: Object?,
    slot: number
): Object?
    local o = battle.spawn_kind("gust", Vec3.zero, params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = alliance, flip
    o.damage = bit32.band(damage, 0xFFFF)
    o.stamina = bit32.rshift(damage, 16)
    local s = o.state :: State
    s.holder = holder
    s.slot = slot
    if holder then
        (holder.state :: any).gusts[slot + 1] = 1
    end
    return o
end

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
    me.collision.region = 0
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CD414`: blow on; end on a stopping hit, off the field, or (unless
-- invisible) on a panel it can't blow over.
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
        elseif bit32.btest(me:param(1), INVISIBLE) then
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
        c.region = 0
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

-- `sub_80CD310`: its look, its speed along the row (the way the first
-- parameter turns it), its registration.
local function init(me: Object)
    local kind = me:param(1)
    if not bit32.btest(kind, INVISIBLE) then
        me.visible = true
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:step()
    local turn = bit32.band(kind, 0x7F)
    sprite:set_flip(bit32.bxor(bit32.band(bit32.bxor(me.alliance, me.flip), 0x7F), kind))
    me.element = ELEMENT
    sprite.palette = int.u8(kind * 2)
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
    me:setup_collision(SELF_TYPE, TARGET_TYPE, if bit32.btest(kind, WEAK) then WEAK_PUSH else PUSH)
    me.collision.self_damage = 0
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CD2EC`.
function gust.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return gust
