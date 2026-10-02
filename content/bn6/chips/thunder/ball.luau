--!strict
-- A thunder ball (the original's attack object #0x2A, `sub_80C93CC`):
-- Thunder's and DarkThnd's. It drifts a panel per 60 ticks (40 if fast),
-- homing: each panel it picks the nearest column with an enemy body
-- (looking ahead first, then back; bottom row up) and goes along its row
-- toward it, or up or down its column when the target is in it; with none
-- it goes straight ahead. It hits with its status (Thunder's paralysis)
-- and bug, for its panels, then bursts. It goes when it hits a body, a
-- neutral object or an attack, leaves the field, or the battle ends. A
-- ball of 12 panels (DarkThnd's) is big and sounds otherwise.
--
-- Its first related slot can hold an object whose header it sets (to 1
-- when it comes, 0 when it goes); Thunder gives none.
--
-- Its look (sprites, animations, collision type, hit modifier, sounds) is a
-- game's: BN6's below (`ball.kind`); another game's kind of it is
-- `ball.make_kind(id, look)` (BN5's).

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local ball = {}

-- What a ball is spawned with (the original's four parameters): fast
-- (Param1 2), the panels it drifts (Param2), the status its hits inflict
-- (Param3, 0: none) and their bug (Param4, 0: none; its argument is 1).
export type Spec = ThunderBall

export type State = {
    fast: boolean,
    -- Panels left (Param2).
    panels: number,
    status: Status?,
    bug: number,
}

-- BN6's look: one sprite, its animations small and big (fast, or
-- DarkThnd's 12 panels; +1 bursting); an attack hitting with modifier 1;
-- its sound, DarkThnd's own.
local SPRITE = asset.sprite("thunder-ball")
local BN6: ThunderBallLook = {
    sprite = SPRITE,
    small_anim = 0,
    big_sprite = SPRITE,
    big_anim = 2,
    is = collision.attack,
    hit_mod = 1,
    sound = asset.sound("erase-man-2"),
    dark_sound = asset.sound("elec-man"),
}
-- The panel count that makes DarkThnd's.
local DARK_PANELS = 0x0C
-- Hits that end it: bodies, neutral objects, attacks.
local ENDING = 0xFF800000
-- The enemy bodies it looks for, by its side.
local TARGETS = { [0] = 0x04000000, [1] = 0x08000000 }
-- Speeds (16.16 per tick) and ticks a panel: normal and fast.
local ROW_SPEED, FAST_ROW_SPEED = 0xAAAB, 0x1_0000
local COLUMN_SPEED, FAST_COLUMN_SPEED = 0x6666, 0x999A
local PANEL_TICKS, FAST_PANEL_TICKS = 0x3C, 0x28

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
    local fast = (me.state :: State).fast
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
-- its panels.
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
    local s = me.state :: State
    local left = s.panels - 1
    s.panels = left
    if bit32.band(left, 0xFF) == 0 then
        me.anim += 1
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    aim(me)
end

-- Out of the fight: its region cleared, then the burst.
local function finish(me: Object)
    me.collision:set_region(nil)
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
    me:highlight_collision_panels()
end

-- `sub_80C940C`.
local function init(me: Object, look: ThunderBallLook)
    local s = me.state :: State
    local dark = s.panels == DARK_PANELS
    local big = s.fast or dark
    local sprite = me.sprite
    sprite:load(if big then look.big_sprite else look.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = if big then look.big_anim else look.small_anim
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
    me:setup_collision(if dark then look.dark_is or look.is else look.is, collision.hits_navis, look.hit_mod)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(sparks.elec)
    if s.status then
        c.status_base = s.status
    end
    if s.bug ~= 0 then
        -- sub_801A4D0: the bug, with argument 1.
        c.bugs = s.bug + 0x100
    end
    c:present()
    aim(me)
    battle.play_sound(if dark then look.dark_sound or look.sound else look.sound)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C93F0`: burst, then gone once the animation ends.
local function burst(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:destroy()
    end
end

-- A thunder ball kind (`id`) of `look`.
function ball.make_kind(id: string, look: ThunderBallLook): Kind
    return define.kind {
        id = id,
        pool = "attack",
        state = { fast = "bool", panels = "u8", status = "status", bug = "u8" },
        -- `sub_80C93CC`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me)
            else
                burst(me)
            end
            if me.active then
                me:update_sprite()
            end
        end,
    }
end

ball.kind = ball.make_kind("thunder/ball", BN6)

-- `sub_80C954C`: a thunder ball for `owner` on (x, y) with the damage word
-- `damage`. Its position is the spawner's registers (the panel Y, the
-- element and a height of 10 pixels) until its init places it; the height
-- stays. (A game's own kind of it: `kind`.)
function ball.spawn(owner: Object, x: number, y: number, element: number, damage: number, spec: Spec, kind: Kind?): Object?
    local o = battle.spawn(kind or ball.kind, Vec3.new(y, element, 0xA_0000))
    if o then
        local s = o.state :: State
        s.fast, s.panels, s.status, s.bug = spec.fast or false, spec.panels, spec.status, spec.bug or 0
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = nil
    end
    return o
end

return ball
