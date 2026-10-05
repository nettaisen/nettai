--!strict
-- A hit's flashes (the original's effect object #0x73, `sub_80E88A0`):
-- SlashCross Beast's lunge (./lunge_slash) flashes the screen and the
-- objects three times, 8 ticks apart (palette flash variant 1, 2 ticks
-- each), when it strikes a body. It shakes the screen too (presentation
-- only). Gone once the battle is over.

local hit_flash = {}

export type State = {
    -- Flashes to go (the original's second parameter).
    flashes: number,
}

-- The flash (`off_80E8934`): variant 1 (the screen and the objects) for 2
-- ticks, neither going on while dimmed nor while paused.
local FLASH_VARIANT, FLASH_TICKS = 1, 2

-- The flashes, and the ticks between them.
local FLASHES, GAP = 3, 8

-- The actions, on its action byte.
local FLASHING, WAITING = 0, 1

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
    local s = me.state :: State
    local flashes = s.flashes - 1
    s.flashes = flashes
    if flashes == 0 then
        me:set_lifecycle("destroy")
        return
    end
    me.action = FLASHING
end

-- `sub_80E88C0`: three flashes to go.
local function init(me: Object)
    (me.state :: State).flashes = FLASHES
    battle.shake_camera(2, 0xC)
    me:set_lifecycle("update")
    tick(me)
end

hit_flash.kind = define.kind {
    id = "slashcross-beast/hit-flash",
    pool = "effect",
    state = { flashes = "u8" },
    -- `sub_80E88A0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- `sub_80E8912`: flashes for `owner`. (It keeps running while paused but
-- not while dimmed. Its position is its spawner's registers, which nothing
-- reads.)
function hit_flash.spawn(owner: Object): Object?
    local o = battle.spawn(hit_flash.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = false
    end
    return o
end

return hit_flash
