--!strict
-- Rock debris (the original's effect object #0x38, `sub_80E46D8`): a chunk
-- thrown up when a rock (./rock) breaks, which flies off with a random
-- speed and puffs into dust where it lands; the stages' boulder
-- (objects/boulder) and the encased bubble (objects/encased-bubble) throw
-- it up too. Only drawn, but it draws the simulation RNG twice.

local effects = require("../../lib/effects")

local debris = {}

export type State = {
    -- Its sprite palette (its Param1: the rock's debris palette).
    palette: number,
}

local SPRITE = asset.sprite("bn6:rock-debris")
-- Gravity, 16.16 a tick.
local GRAVITY = 0x14000

-- `sub_80E4790`: a dust puff (effect 1, the splash's look), and gone.
local function land(me: Object)
    battle.effect(me.pos, effects.splash)
    me:free()
end

-- `sub_80E475C`: move, fall, and land at height 0.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    local z = int.i32(p.z + v.z)
    if z < 0 then
        z = 0
    end
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, z)
    me.vel = Vec3.new(v.x, v.y, v.z - GRAVITY)
    if z == 0 then
        land(me)
    end
end

-- `sub_80E46F8`: one of two chunk shapes, and a random velocity: up to 3.5
-- pixels a tick sideways and in depth, 6 to 13.5 up.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(bit32.band(battle.rng(), 1))
    sprite:step()
    sprite.shadow = "with_sprite"
    sprite.palette = (me.state :: State).palette
    local r = battle.rng()
    local function nibble(shift: number): number
        return bit32.band(bit32.rshift(r, shift), 0xF)
    end
    me.vel = Vec3.new((nibble(0) - 7) * 0x8000, (nibble(4) - 7) * 0x8000, (nibble(8) + 12) * 0x8000)
    me.lifecycle = "update"
    fly(me)
end

debris.kind = define.kind {
    id = "bn6:rockcube/debris",
    pool = "effect",
    state = { palette = "u8" },
    -- `sub_80E46D8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            fly(me)
        else
            land(me)
        end
    end,
}

-- `sub_80E47A4`: a chunk at `pos` in sprite palette `palette`.
function debris.spawn(pos: Vec3, palette: number): Object?
    local o = battle.spawn(debris.kind, pos)
    if o then
        (o.state :: State).palette = palette
    end
    return o
end

return debris
