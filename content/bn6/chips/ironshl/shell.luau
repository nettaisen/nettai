--!strict
-- An iron shell (the original's attack object #0x34, `sub_80CA6B8`):
-- IronShl's and ParaShl's. It slides from its panel to the opponent's back
-- column (four panels for ParaShl's), hitting what it meets; after a hit
-- its region is off until it reaches another panel. There it stops for 8
-- ticks (16 for ParaShl's) with its region back on, its bumps' times; then
-- an IronShl's flies up and away, a ParaShl's turns and slides back. It
-- bursts off solid ground.
--
-- It hits with modifier 3 (knocking back) while between the side columns,
-- 1 (not) when thrown onto a side column, and from when it reaches the
-- opponent's back column, where it no longer pushes.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local shell = {}

-- How a shell slides (`IronShell`, types.d.luau).
export type Spec = IronShell

export type State = {
    -- Its palette and speed (the original's first parameter picks them
    -- from `byte_80CA7A0`), its bumps (the second), and whether it is
    -- ParaShl's (the third).
    palette: number,
    speed: number,
    bumps: number,
    para: boolean,
    -- The panel (x | y << 8) it last hit on (ExtraVars+0).
    hit_panel: number,
    -- Its hit modifier dropped at the far column (ExtraVars+4).
    softened: boolean,
}

local SPRITE, ANIM = asset.sprite("bn6:iron-shell"), 3
local SOUND = asset.sound("bn6:iron-shell")
-- A panel's width, 16.16.
local PANEL_WIDTH = 0x28_0000
-- The rise as it flies off, 16.16 per tick.
local FLY_UP = 0x4_0000
-- The hit modifiers.
local PUSHES, STILL = 3, 1
local SOLID = 0x10

-- The actions.
local SLIDE, BUMP, FLY = 0, 4, 8

-- The puff it bursts in, 16 pixels above it.
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.puff)
end

-- The panel as the game's halfword (x | y << 8).
local function panel_word(me: Object): number
    return me.panel_x + me.panel_y * 0x100
end

-- The opponent's back column.
local function far_column(me: Object): number
    return bit32.bxor(me.alliance, 1) * 5 + 1
end

-- `sub_80CA834`: slide; at the far column the push goes; stop when the
-- slide's time is up.
local function slide(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local s = me.state :: State
    if not s.softened and me.panel_x == far_column(me) then
        s.softened = true
        me:reset_collision_types(collision.breaking, collision.hits_navis, STILL)
    end
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me:set_coordinates_from_panel()
    me.timer = if s.para then 0x10 else 8
    me:set_action(BUMP)
end

-- `sub_80CA88E`: stand, the region back on each 8 ticks, its bumps'
-- times; then fly off (IronShl) or turn back (ParaShl).
local function bump(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.collision:set_region(regions.single)
    local s = me.state :: State
    local left = s.bumps - 1
    s.bumps = left
    if int.u8(left) ~= 0 then
        me.timer = 8
        return
    end
    if s.para then
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
        me.flip = bit32.bxor(me.flip, 1)
        me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    else
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, FLY_UP)
    end
    me:reset_collision_types(collision.breaking, collision.hits_navis, PUSHES)
    me:set_action(FLY)
end

-- `sub_80CA8E2`: fly on; gone off the field.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80CA7AC`: resolve its hits; move; burst off solid ground; its
-- region comes back on a new panel.
local function tick(me: Object)
    local c = me.collision
    local s = me.state :: State
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        s.hit_panel = panel_word(me)
        c:set_region(nil)
    end
    local a = me.action
    if a == SLIDE then
        slide(me)
    elseif a == BUMP then
        bump(me)
    elseif a == FLY then
        fly(me)
    else
        error(string.format("iron shell action %#x reads past off_80CA828", a))
    end
    if not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        puff(me)
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local highlight = true
    if c.region == nil then
        if panel_word(me) == s.hit_panel then
            highlight = false
        else
            c:set_region(regions.single)
        end
    end
    if highlight then
        field.highlight(me.panel_x, me.panel_y)
    end
    me:update_sprite()
    c:present()
end

-- `sub_8011504`: the sprite with a ground shadow, then collision with hit
-- modifier `hit_mod` (false when no slot was free).
local function setup(me: Object, hit_mod: number): boolean
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = ANIM
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        return false
    end
    me:setup_collision(collision.breaking, collision.hits_navis, hit_mod)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    return true
end

-- `sub_80CA6D8`: on solid ground, its speed and a slide to the far column.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local s = me.state :: State
    local x, y = me.panel_x, me.panel_y
    local ok = field.valid(x, y) and bit32.btest(field.flags(x, y), SOLID)
    if ok then
        -- It knocks back only thrown onto a middle column.
        local hit_mod = if s.para or x == 1 or x >= 6 then STILL else PUSHES
        ok = setup(me, hit_mod)
    end
    if not ok then
        puff(me)
        me:free()
        return
    end
    me.visible = true
    me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.sprite.palette = s.palette
    local v = me.vel
    me.vel = Vec3.new(me.facing * s.speed, v.y, v.z)
    me.collision:set_hit_effect(sparks.breaking)
    local panels = if s.para then 4 else math.abs(far_column(me) - x)
    me.timer = int.tdiv(PANEL_WIDTH * panels, s.speed) + 1
    me.collision:present()
    battle.play_sound(SOUND)
    me.lifecycle = "update"
    tick(me)
end

shell.kind = define.kind {
    id = "bn6:ironshl/shell",
    pool = "attack",
    state = {
        palette = "u8",
        speed = "i32",
        bumps = "u8",
        para = "bool",
        hit_panel = "u32",
        softened = "bool",
    },
    -- `sub_80CA6B8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80CA90E`: a shell for `owner` on (x, y), with the damage word
-- `damage`, sliding as `spec` says. Its position is the spawner's
-- registers (the panel Y, the element and 0) until its init places it.
function shell.spawn(owner: Object, x: number, y: number, element: number, damage: number, spec: Spec): Object?
    local o = battle.spawn(shell.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.palette, s.speed, s.bumps, s.para = spec.palette, spec.speed, spec.bumps, spec.para or false
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return shell
