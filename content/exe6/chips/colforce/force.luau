--!strict
-- ColForce (the original's effect object #0x80, `sub_80E9460`) and the
-- chip's effect (instant chip effect 15, `sub_80E94DC`): from the other
-- side's back column it looks, column by column toward the user, for the
-- first column with any of the other side's navis or objects in front of
-- free panels; behind each of them a soldier (soldier.luau) drops in, 10
-- ticks apart. Every 50 ticks it looks again from that column on, until a
-- look finds nothing or it runs off the field. It stands still while
-- dimmed and has no sprite.

local soldier = require("./soldier")

local force = {}

-- What a soldier's panel must be (`byte_80E955C`, by side: required and
-- forbidden flags): a free panel of the other side's, by side.
local REQUIRE = { [0] = 0x10, [1] = 0x30 }
local FORBID = { [0] = 0x03800020, [1] = 0x03800000 }
local LOOK_TICKS = 0x32
local SOLDIER_DELAY = 0xA

-- `sub_80E94F8`: the next column toward the user with soldiers' panels
-- in it; a soldier on each, and it moves there. How many.
local function look(me: Object): number
    local side = me.alliance
    -- object_getEnemyDirection, the other way.
    local toward = -(1 - 2 * side)
    local require, forbid = REQUIRE[side], FORBID[side]
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local n = 0
    local ahead = 1
    while true do
        local x = me.panel_x + toward * ahead
        for y = 1, 3 do
            if x < 1 or x > 6 then
                return n
            end
            if field.check(x, y, require, forbid) then
                soldier.spawn(me, x, y, me.element, SOLDIER_DELAY * n, damage)
                n += 1
            end
        end
        if n ~= 0 then
            me.panel_x = x
            return n
        end
        ahead += 1
    end
end

-- `sub_80E94B4`.
local function tick(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    if look(me) ~= 0 and me.panel_x >= 1 and me.panel_x <= 6 then
        me.timer = LOOK_TICKS
        return
    end
    me:set_lifecycle("destroy")
end

force.kind = define.kind {
    id = "colforce/force",
    pool = "effect",
    -- `sub_80E9460`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80E9480: the other side's back column.
            me.panel_x = if me.alliance == 0 then 6 else 1
            me.timer = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            -- sub_80E9498
            if battle.over() then
                me:set_lifecycle("destroy")
            elseif not battle.dimmed() then
                tick(me)
            end
        else
            me:free()
        end
    end,
}

-- `sub_80E94DC`, instant chip effect 15: the force of the user's. (It
-- spawns at the registers' values; its init sets its column.)
function force.instant(user: Object, spec: InstantChipSpec)
    local o = battle.spawn(force.kind, Vec3.new(spec.panel_y, spec.element, spec.z))
    if o then
        o.element = spec.element
        o.related1 = user
        o:set_damage_word(spec.damage)
        o.alliance, o.flip = user.alliance, user.flip
    end
end

return force
