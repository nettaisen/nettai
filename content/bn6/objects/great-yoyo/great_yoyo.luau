--!strict
-- GreatYo's controller (effect object #0x70, `sub_80E857C`): invisible. On
-- its first update it sends out three yoyos (objects/yoyo) with its own
-- parameters and damage: on its panel's row below, above and on it, and
-- holds them. Once all three are gone it clears its navi's first related
-- object (which held it) and ends.

local yoyo = require("../yoyo/yoyo")

export type State = {
    -- The yoyos it holds (ExtraVars+0, +4 and +8); the yoyos clear these
    -- when they go.
    middle: Object?,
    below: Object?,
    above: Object?,
}

local great_yoyo = {
    state = { middle = "object", below = "object", above = "object" },
}

-- `sub_80E85FA`: a controller for `owner` on (x, y) at height `z` with the
-- damage word `damage` and the chip's `params`. Its first related object
-- is the navi, whose first related object holds it. Its position stays
-- the spawner's registers (the panel Y, the element and the height).
function great_yoyo.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    params: { number }
): Object?
    local o = battle.spawn_kind("great-yoyo", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80E862E`: one of its yoyos, on row `y`, held in `slot`.
local function send(me: Object, y: number, slot: "middle" | "below" | "above"): Object?
    local params = { me:param(1), me:param(2), me:param(3), me:param(4) }
    local damage = me.damage + me.stamina * 0x10000
    return yoyo.spawn(me, me.panel_x, y, me.element, me.pos.z, damage, params, slot)
end

-- `sub_80E85A8`: send the yoyos; end once they're all gone.
local function tick(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local y = me.panel_y
        s.below = send(me, y + 1, "below")
        s.above = send(me, y - 1, "above")
        s.middle = send(me, y, "middle")
        me.phase_init = 4
    end
    if s.middle == nil and s.below == nil and s.above == nil then
        local navi = me.related1
        if navi then
            navi.related1 = nil
        end
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E857C`.
function great_yoyo.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        -- sub_80E859C
        me:set_lifecycle("update")
        tick(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return great_yoyo
