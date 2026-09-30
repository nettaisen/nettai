--!strict
-- SlashCross Beast's lunge's slash (attack object #0xB1, `sub_80DB8CC`): a
-- hit on its panel (region 0x0B) that resolves as soon as it is placed. A
-- body it strikes gets the hit's flashes (objects/hit-flash). It keeps
-- going while its timer lasts, which nothing sets, so it ends at once.

local hit_flash = require("../hit-flash/hit_flash")

local lunge_slash = {}

-- What its hit struck that flashes: a body of either side.
local BODY = 0x0C00_0000

-- `sub_80DB970`: a slash for `owner` on (x, y) with `element` and the
-- damage word `damage`. (Its position is the spawner's registers, Y, the
-- element and 0, until its init places it.)
function lunge_slash.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn_kind("lunge-slash", Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- Done: its region, its collision, and the slot go.
local function finish(me: Object)
    local c = me.collision
    c.region = 0
    c:free()
    me:free()
end

-- `sub_80DB934`: resolve the hit; flashes for a body it struck.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local hit = c.hit_flags
    if hit ~= 0 then
        if bit32.btest(hit, BODY) then
            hit_flash.spawn(me)
        end
        return finish(me)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        return finish(me)
    end
    c:present()
end

-- `sub_80DB8EC`: on its panel (if it is on the field), with its hit.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local x, y = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(x, y, me.pos.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(7, 5, 3)
    me.collision.region = 0x0B
    me.collision.hit_effect = 0xFF
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80DB8CC`.
function lunge_slash.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return lunge_slash
