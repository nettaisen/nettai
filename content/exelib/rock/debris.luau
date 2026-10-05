--!strict
-- Rock debris, as EXE5 and EXE6 share it (EXE6's effect object #0x38, `sub_80E46D8`; EXE5's #0x38, 0x080E46F4, the same
-- code): a chunk thrown up when a rock (@exelib/rock/rock) breaks, which flies off with a random speed and puffs into
-- dust where it lands; the stages' boulder (@exelib/boulder/boulder) and EXE6's encased bubble throw it up too. Only
-- drawn, but it draws the simulation RNG twice.
--
-- Each game makes its own kind (`debris.make_kind`) with its look: the chunks' sprite and the dust they land in.

local debris = {}

export type State = {
    -- Its sprite palette (its Param1: the rock's debris palette).
    palette: number,
}

-- Gravity, 16.16 a tick.
local GRAVITY = 0x14000

-- A game's debris kind: its key, the chunks' sprite (two shapes, animations 0 and 1) and the dust a chunk lands in
-- (effect 1, the splash's look).
function debris.make_kind(id: string, sprite: SpriteAsset, dust: Effect): Kind
    -- `sub_80E4790`: a dust puff, and gone.
    local function land(me: Object)
        battle.effect(me.pos, dust)
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

    -- `sub_80E46F8`: one of two chunk shapes, and a random velocity: up to 3.5 pixels a tick sideways and in depth,
    -- 6 to 13.5 up.
    local function init(me: Object)
        me.visible = true
        local s = me.sprite
        s:load(sprite)
        s:set_animation(bit32.band(battle.rng(), 1))
        s:step()
        s.shadow = "with_sprite"
        s.palette = (me.state :: State).palette
        local r = battle.rng()
        local function nibble(shift: number): number
            return bit32.band(bit32.rshift(r, shift), 0xF)
        end
        me.vel = Vec3.new((nibble(0) - 7) * 0x8000, (nibble(4) - 7) * 0x8000, (nibble(8) + 12) * 0x8000)
        me.lifecycle = "update"
        fly(me)
    end

    return define.kind {
        id = id,
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
end

-- `sub_80E47A4`: a chunk (of `kind`, a game's) at `pos` in sprite palette `palette`.
function debris.spawn(kind: Kind, pos: Vec3, palette: number): Object?
    local o = battle.spawn(kind, pos)
    if o then
        (o.state :: State).palette = palette
    end
    return o
end

return debris
