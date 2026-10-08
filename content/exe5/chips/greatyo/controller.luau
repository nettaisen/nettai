--!strict
-- GreatYo's controller (effect object #0x70, 0x080E89C4): what the YoYo throw (action 0x4A with subtype 1, 0x080EF256)
-- sends out in place of a yoyo, held in the thrower's first related object. It throws three yoyos (chips/yoyo/yoyo,
-- 0x080D0346) from its panel: a follower on the row below and one on the row above (its parameters with Param2 2,
-- 0x080E8A84), then the leader on its own row (its parameters, Param2 1: 0x080E8A74), at its height, with its element
-- and damage word; once all three are gone, it lets the thrower go on (clearing what holds it) and is freed.

local yoyo = require("../yoyo/yoyo")

local controller = {}

-- How a yoyo flies (chips/yoyo/yoyo's `Spec`).
type YoyoSpec = { panels: number, rounds: number }

export type State = {
    -- The yoyos it holds (+0x60, +0x64, +0x68): what each clears when it goes. (The middle one is the leader, the
    -- others the followers: the yoyo knows by its slot.)
    middle: Object?,
    below: Object?,
    above: Object?,
    spec: YoyoSpec,
}

-- 0x080E89F0: the three yoyos; then wait until all are gone.
local function update(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local x, y, z = me.panel_x, me.panel_y, me.pos.z
        local damage = int.u32(me.damage + me.stamina * 0x10000)
        s.below = yoyo.spawn(me, x, y + 1, me.element, z, damage, s.spec, "below")
        s.above = yoyo.spawn(me, x, y - 1, me.element, z, damage, s.spec, "above")
        s.middle = yoyo.spawn(me, x, y, me.element, z, damage, s.spec, "middle")
        me.phase_init = 4
    end
    if s.middle == nil and s.below == nil and s.above == nil then
        -- (The original clears what held it, the thrower's first related object, whatever that is now.)
        local thrower = me.related1
        if thrower then
            thrower.related1 = nil
        end
        -- (0x080E8A3E: a word store, its action and phase cleared with it.)
        me:set_lifecycle("destroy")
    end
end

controller.kind = new.kind {
    id = "greatyo/controller",
    pool = "effect",
    state = { middle = "object", below = "object", above = "object", spec = "record:exe5-yoyo" },
    -- 0x080E89C4.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- (0x080E89E8: a word store.)
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            me:free()
        end
    end,
}

-- 0x080E8A42: a controller for `thrower` on (x, y), `z` up, with the damage word, its yoyos flying as `spec` says.
-- (Its position is the spawner's registers: Y, 0 and the height.)
function controller.spawn(thrower: Object, x: number, y: number, z: number, damage: number, spec: YoyoSpec): Object?
    local o = battle.spawn(controller.kind, Vec3.new(y, 0, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = thrower.alliance, thrower.flip
        o.related1 = thrower
        local s = o.state :: State
        s.spec = spec
    end
    return o
end

return controller
