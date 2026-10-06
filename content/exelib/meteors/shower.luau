--!strict
-- What a shower of meteors aims at, as EXE5 and EXE6 share it (EXE6's effect
-- object #0x46, `sub_80E59E8`, lib/instant/meteor_shower; EXE5's #0x46,
-- 0x080E58EC): one of the other side's navis still standing, and its panel
-- or a neighbor of it.

local shower = {}

-- Whether to pick a panel next to the target, by a draw's low four bits
-- (`byte_80E5B50`, running into `byte_80E5B58`), and the neighbors
-- (`byte_80E5B58`, its last pair the next routine's code).
local NEAR = { [0] = false, true, true, false, false, false, false, false, false, true, false, true, true, false, true, false }
local NEIGHBORS = { { 0, -1 }, { 0, 1 }, { -1, 0 }, { 1, 0 } }

-- `sub_80E5A8A`: one of the other side's navis still standing, at random
-- (a draw only if there is one).
function shower.pick(me: Object): Object?
    local standing: { Object } = {}
    for _, o in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        if o.hp ~= 0 then
            table.insert(standing, o)
        end
    end
    if #standing == 0 then
        return nil
    end
    return standing[battle.rng_positive() % #standing + 1]
end

-- `sub_80E5AD6`: the target's panel, or on some draws a neighbor of it on
-- the field (another draw picks which).
function shower.aim(target: Object?): (number, number)
    if target == nil then
        error("meteor shower without a target (sub_80E5AD6 reads a panel through a null pointer)")
    end
    local x, y = target.panel_x, target.panel_y
    if not NEAR[bit32.band(battle.rng_positive(), 0xF)] then
        return x, y
    end
    local near = {}
    for _, d in NEIGHBORS do
        local nx, ny = x + d[1], y + d[2]
        if field.valid(nx, ny) then
            table.insert(near, bit32.bor(bit32.band(nx, 0xFF), bit32.lshift(ny, 4)))
        end
    end
    if #near == 0 then
        return x, y
    end
    local b = bit32.band(near[battle.rng_positive() % #near + 1], 0xFF)
    return bit32.band(b, 7), bit32.rshift(b, 4)
end

return shower
