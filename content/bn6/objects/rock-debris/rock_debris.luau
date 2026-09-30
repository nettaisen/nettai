--!strict
-- Rock debris (effect object #0x38, `sub_80E46D8`): a chunk thrown up when
-- a rock breaks, which flies off with a random speed and puffs into dust
-- where it lands. Only drawn, but it draws the simulation RNG twice.

local rock_debris = {}

local SPRITE = "10-01"
-- The dust puff it lands in (effect #0 look 1).
local DUST = 1
-- Gravity, 16.16 a tick.
local GRAVITY = 0x14000

-- `sub_80E47A4`: a chunk at `pos` in sprite palette `palette`.
function rock_debris.spawn(pos: Vec3, palette: number): Object?
    return battle.spawn_kind("rock-debris", pos, { palette })
end

-- `sub_80E4790`: a dust puff, and gone.
local function land(me: Object)
    battle.effect(me.pos, DUST)
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
    sprite.palette = me:param(1)
    local r = battle.rng()
    local function nibble(shift: number): number
        return bit32.band(bit32.rshift(r, shift), 0xF)
    end
    me.vel = Vec3.new((nibble(0) - 7) * 0x8000, (nibble(4) - 7) * 0x8000, (nibble(8) + 12) * 0x8000)
    me.lifecycle = "update"
    fly(me)
end

-- `sub_80E46D8`.
function rock_debris.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        fly(me)
    else
        land(me)
    end
end

return rock_debris
