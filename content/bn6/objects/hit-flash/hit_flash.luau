--!strict
-- A hit's flashes (effect object #0x73, `sub_80E88A0`): SlashCross Beast's
-- lunge flashes the screen and the objects three times, 8 ticks apart
-- (palette flash variant 1, 2 ticks each), when it strikes a body. It
-- shakes the screen too (presentation only). Gone once the battle is over.

local hit_flash = {}

-- The flash (`off_80E8934`): variant 1 (the screen and the objects) for 2
-- ticks, neither going on while dimmed nor while paused.
local FLASH_VARIANT, FLASH_TICKS = 1, 2

-- The flashes, and the ticks between them.
local FLASHES, GAP = 3, 8

-- The actions, on its action byte.
local FLASHING, WAITING = 0, 1

-- `sub_80E8912`: flashes for `owner`. (It keeps running while paused but
-- not while dimmed.)
function hit_flash.spawn(owner: Object): Object?
    local o = battle.spawn_kind("hit-flash", Vec3.zero)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = false
    end
    return o
end

-- `sub_80E88D8`: a flash, then 8 ticks; three times.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if me.action == FLASHING then
        me.action = WAITING
        battle.palette_flash(FLASH_VARIANT, FLASH_TICKS)
        me.timer = GAP
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local flashes = me:param(2) - 1
    me:set_param(2, flashes)
    if flashes == 0 then
        me:set_lifecycle("destroy")
        return
    end
    me.action = FLASHING
end

-- `sub_80E88C0`: three flashes to go (Param2).
local function init(me: Object)
    me:set_param(2, FLASHES)
    -- (A camera shake: presentation only.)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E88A0`.
function hit_flash.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return hit_flash
