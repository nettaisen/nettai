--!strict
-- A thunder ball (attack object #0x2A, `sub_80C93CC`): Thunder's and
-- DarkThnd's. It drifts a panel per 60 ticks (40 with Param1 2), homing:
-- each panel it picks the nearest column with an enemy body (looking
-- ahead first, then back; bottom row up) and goes along its row toward
-- it, or up or down its column when the target is in it; with none it
-- goes straight ahead. It hits with status Param3 (Thunder's paralysis)
-- and bug Param4, for Param2 panels, then bursts. It goes when it hits a
-- body, a neutral object or an attack, leaves the field, or the battle
-- ends.
--
-- Its first related slot can hold an object whose header it sets (to 1
-- when it comes, 0 when it goes); Thunder gives none.

local thunder_ball = {}

local SPRITE = "10-16"
local SELF_TYPE, TARGET_TYPE, HIT_MOD, HIT_EFFECT = 4, 5, 1, 3
-- Its animations: small, big (Param1 2, or DarkThnd's 12 panels); +1
-- bursting.
local SMALL, BIG = 0, 2
local SOUND_THUNDER, SOUND_DARK = 0xBA, 0xC6
-- The panel count that makes DarkThnd's.
local DARK_PANELS = 0x0C
-- Hits that end it: bodies, neutral objects, attacks.
local ENDING = 0xFF800000
-- The enemy bodies it looks for, by its side.
local TARGETS = { [0] = 0x04000000, [1] = 0x08000000 }
-- Speeds (16.16 per tick) and ticks a panel: normal and fast (Param1 2).
local ROW_SPEED, FAST_ROW_SPEED = 0xAAAB, 0x1_0000
local COLUMN_SPEED, FAST_COLUMN_SPEED = 0x6666, 0x999A
local PANEL_TICKS, FAST_PANEL_TICKS = 0x3C, 0x28

-- `sub_80C954C`: a thunder ball for `owner` on (x, y) with the chip's
-- parameters and the damage word `damage`. Its position is the spawner's
-- registers (the panel Y, the element and a height of 10 pixels) until
-- its init places it; the height stays.
function thunder_ball.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("thunder-ball", Vec3.new(y, element, 0xA_0000), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = nil
    end
    return o
end

-- `sub_80C9620` (ahead, `dir` 1) and `sub_80C9640` (back, -1): from its
-- column, the first column with a target body, bottom row up: (x, y), or
-- 0 for none.
local function search(me: Object, dir: number): (number, number)
    local x = me.panel_x
    -- The search ahead stops at column 7, the one back at column 0; from
    -- past them the game's search runs until the column wraps around.
    local stop = if dir > 0 then 7 else 0
    if (dir > 0 and x >= stop) or (dir < 0 and x == stop) then
        error(string.format("thunder ball: a target search from column %d never ends", x))
    end
    local targets = TARGETS[me.alliance]
    while x ~= stop do
        for y = 3, 1, -1 do
            if bit32.btest(field.flags(x, y), targets) then
                return x, y
            end
        end
        x += dir
    end
    return 0, 0
end

-- `sub_80C95EC`: the nearest target, looking the way it faces first.
local function find_target(me: Object): (number, number)
    local ahead = if bit32.bxor(me.alliance, me.flip) == 0 then 1 else -1
    local x, y = search(me, ahead)
    if x == 0 then
        x, y = search(me, -ahead)
    end
    return x, y
end

-- `sub_80C956C`: head for the target for a panel's time: along its row,
-- or along its column when the target is in it; straight ahead without
-- one.
local function aim(me: Object)
    local fast = me:param(1) == 2
    local tx, ty = find_target(me)
    local vertical = false
    local dir = me.facing
    if tx ~= 0 then
        if ty ~= me.panel_y and tx == me.panel_x then
            vertical = true
            dir = if ty >= me.panel_y then 1 else -1
        else
            dir = if tx >= me.panel_x then 1 else -1
        end
    end
    if vertical then
        me.vel = Vec3.new(0, dir * (if fast then FAST_COLUMN_SPEED else COLUMN_SPEED), me.vel.z)
    else
        me.vel = Vec3.new(dir * (if fast then FAST_ROW_SPEED else ROW_SPEED), 0, me.vel.z)
    end
    me.timer = if fast then FAST_PANEL_TICKS else PANEL_TICKS
end

-- `sub_80C950C`: drift; after each panel's time, aim again, or burst after
-- Param2 panels.
local function drift(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = t
    if bit32.band(t, 0xFFFF) ~= 0 then
        return
    end
    local left = me:param(2) - 1
    me:set_param(2, left)
    if bit32.band(left, 0xFF) == 0 then
        me.anim += 1
        me.collision.region = 0
        me:set_lifecycle("destroy")
        return
    end
    aim(me)
end

-- `object_highlightCurrentCollisionPanels`: light the panels of its
-- region.
local function highlight(me: Object)
    local c = me.collision
    local panels = data.regions[c.region]
    if panels == nil then
        error(string.format("thunder ball: region %d reads past PanelOffsetListsPointerTable", c.region))
    end
    local dir = me.facing
    for _, p in panels do
        field.highlight(c.panel_x + dir * p[1], c.panel_y + p[2])
    end
end

-- Out of the fight: its region cleared, then the burst.
local function finish(me: Object)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C94BC`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or bit32.btest(c.hit_flags, ENDING) then
        return finish(me)
    end
    if me.action ~= 0 then
        error(string.format("thunder ball action %#x reads past its table", me.action))
    end
    drift(me)
    if not field.valid(me.panel_x, me.panel_y) then
        return finish(me)
    end
    c:present()
    highlight(me)
end

-- `sub_80C940C`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = if me:param(1) == 2 or me:param(2) == DARK_PANELS then BIG else SMALL
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    local c = me.collision
    c.region = 1
    c.hit_effect = HIT_EFFECT
    if me:param(3) ~= 0 then
        c.status_base = me:param(3)
    end
    if me:param(4) ~= 0 then
        -- sub_801A4D0: the bug, with argument 1.
        c.bugs = me:param(4) + 0x100
    end
    c:present()
    aim(me)
    battle.play_sound(if me:param(2) == DARK_PANELS then SOUND_DARK else SOUND_THUNDER)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C93F0`: burst, then gone once the animation ends.
local function burst(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:destroy()
    end
end

-- `sub_80C93CC`.
function thunder_ball.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        burst(me)
    end
    if me.active then
        me:update_sprite()
    end
end

return thunder_ball
