--!strict
-- What the dragons' head (objects/dragon-head) and body segments
-- (objects/dragon-body) share: they rise out of the row above the field,
-- dive down a column to the bottom row, swim a panel forward in a dip,
-- and climb back out, the segments following the head's path a few
-- ticks behind. Param1 is the dragon's kind (the chip's subtype: heat,
-- elec, aqua, wood).

local dragon = {}

dragon.SPRITE = "04-10"

-- By kind (Param1): how fast it moves, 16.16 (`byte_80DE3DC`).
local SPEEDS = { [0] = 0x4_0000, 0x4_0000, 0x4_0000, 0x4_0000, 0x4_0000, 0x4_0000 }
-- A panel's width, 16.16.
local PANEL_WIDTH = 0x28_0000
-- How far it climbs past the top row before it is gone, by leg
-- (`byte_80DE7C0`, `byte_80DE3D0`), 16.16.
local EXIT = { [0] = 0x40_0000, 0x10_0000 }
-- Which way the dip goes, by leg (`byte_80DE704`, `byte_80DE384`).
dragon.DIP = { [0] = 1, -1 }
-- How deep the dip is, in pixels at the sine's peak.
local DIP_DEPTH = 12
-- How far below the field's plane it is drawn, whole pixels.
local HEIGHT = 0x24

-- The legs of its path: down a column, then up one.
dragon.DOWN, dragon.UP = 0, 1

-- The kind's speed.
function dragon.speed(me: Object): number
    local kind = me:param(1)
    local speed = SPEEDS[kind]
    if not speed then
        error(string.format("dragon kind %d reads past its speed table", kind))
    end
    return speed
end

-- Its look: the dragon sprite at animation `anim` in the kind's colours,
-- on its panel, 36 pixels lower (the game subtracts from Z's whole part
-- with a halfword store, which keeps the fraction).
function dragon.place(me: Object, anim: number)
    local sprite = me.sprite
    sprite:load(dragon.SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 3 * me:param(1)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z - HEIGHT * 0x10000)
end

-- Its motion (`sub_80DE67E`, `sub_80DE30E`): down its column and toward
-- the enemy at the kind's speed. Returns the ticks a panel takes and the
-- dip's angle step (0 if it never moves).
function dragon.start(me: Object): (number, number)
    local speed = dragon.speed(me)
    me.vel = Vec3.new(me.facing * speed, speed, me.vel.z)
    local per_panel = int.tdiv(PANEL_WIDTH, speed)
    local step = if per_panel ~= 0 then int.tdiv(0x80, per_panel) else 0
    return per_panel, step
end

-- `sub_80DE7A0`, `sub_80DE3B0`: the ticks it keeps climbing past the top
-- row on leg `leg`.
function dragon.exit_ticks(me: Object, leg: number): number
    return int.tdiv(EXIT[leg], math.abs(me.vel.y))
end

-- `sub_80DE6D0`, `sub_80DE350`: its height on the dip at `angle` (the
-- game reads the sine table with the unwrapped angle), from `base_y`.
function dragon.dip(me: Object, angle: number, base_y: number, leg: number)
    local sine = data.rules.sine[angle]
    if not sine then
        error(string.format("dragon dip angle %d reads past the sine table", angle))
    end
    local depth = int.u16(sine) * DIP_DEPTH * 0x100 * dragon.DIP[leg]
    local p = me.pos
    me.pos = Vec3.new(p.x, base_y + depth, p.z)
end

-- `sub_80DE70C`, `sub_80DE38C`: a splash where it dives in or comes out
-- (not when `leg` is 0), facing its way.
function dragon.splash(me: Object, leg: number)
    if leg == 0 then
        return
    end
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, 0), 2, bit32.bxor(me.alliance, me.flip))
end

return dragon
