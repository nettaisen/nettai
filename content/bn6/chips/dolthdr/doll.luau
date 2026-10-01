--!strict
-- DolThdr's doll (attack object #0x82, `sub_80D5740`): it appears at its
-- user's hand, and on its first tick strikes thunder columns (attack
-- object #0x8B, objects/thunder-column) on every panel of its row from its
-- own to the field's edge, each lasting the thunder's ticks, as the doll
-- does. When it registers (a spawner that passes a slot, the original's
-- first parameter: no chip's) it keeps itself in its user's slot, stands
-- one pixel up and a panel further ahead, and ends its columns early
-- (their timers to 1) once its user clears that slot.

local slot = require("../../lib/slot")
local thunder_column = require("../../objects/thunder-column/thunder_column")

local doll = {}

export type State = {
    -- The thunder columns it struck (`+0x64`), which register here.
    columns: { Object? },
    -- The user's slot it registers in, if it registers.
    slot: Slot,
    registers: boolean,
    -- The thunder's ticks (the original's second parameter).
    ticks: number,
}

local SPRITE = asset.sprite("thunder-doll")
-- The user's hand (attach point 0x1D), and a panel's width in pixels.
local HAND, PANEL = 0x1D, 0x28

-- The user's slot (`+0x60`), for a doll that registers.
local function user_slot(me: Object): Slot
    local s = me.state :: State
    if not s.registers then
        error("DolThdr's doll (sub_80D5764) has no slot to register in")
    end
    return s.slot
end

-- `sub_80D584E`: on its first tick, a column on each panel of its row
-- from its own to the edge.
local function strike(me: Object)
    local s = me.state :: State
    if me.timer ~= s.ticks then
        return
    end
    local dir = me.facing
    local x = me.panel_x
    local link = 1
    while true do
        -- `sub_80D5874`.
        local column = thunder_column.spawn(
            me,
            x,
            me.panel_y,
            me.element,
            thunder_column.plain,
            me.damage + me.stamina * 0x10000,
            me,
            link
        )
        if column then
            column.timer = s.ticks
        end
        link += 1
        x += dir
        if x == 0 or x > 6 then
            return
        end
        if link > 6 then
            -- Only from column 0 going left: the game strikes on forever.
            error("DolThdr's doll (sub_80D584E) strikes on past the field's edge forever")
        end
    end
end

-- Gone; one that registered leaves its user's slot.
local function finish(me: Object)
    if (me.state :: State).registers then
        slot.fill(me.related1 :: Object, user_slot(me), nil)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80D5806`.
local function tick(me: Object)
    strike(me)
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        return finish(me)
    end
    local s = me.state :: State
    if not s.registers then
        return
    end
    if slot.occupied(me.related1 :: Object, user_slot(me)) then
        return
    end
    -- Its user let go: its columns end on their next tick.
    for i = 1, 5 do
        local c = s.columns[i]
        if c then
            c.timer = 1
        end
    end
    finish(me)
end

-- `sub_80D5764`: at the user's hand, back a panel (or, registering, a
-- panel further ahead and a pixel up).
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = 0
    me.timer = s.ticks
    me:set_coordinates_from_panel()
    local user = me.related1 :: Object
    local hx, hz = user:attach_point(HAND)
    local p = me.pos
    local x, y, z = p.x + hx * 0x10000, p.y, p.z + hz * 0x10000
    if s.registers then
        slot.fill(user, user_slot(me), me)
        y -= 0x10000
        z -= 0x10000
        x += me.facing * PANEL * 0x10000
    else
        x -= me.facing * PANEL * 0x10000
    end
    me.pos = Vec3.new(x, y, z)
    me:set_lifecycle("update")
end

doll.kind = define.kind {
    id = "dolthdr/doll",
    pool = "attack",
    state = { columns = "object[6]", slot = slot.TYPE, registers = "bool", ticks = "u8" },
    -- `sub_80D5740`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80D5890`: a doll for `user` on (x, y) whose thunder lasts `ticks`,
-- with the damage word `damage`; `which` is the user's slot it registers
-- in, if it registers. Its position is the spawner's registers (y, the
-- element, 0) until its init places it.
function doll.spawn(user: Object, x: number, y: number, element: number, ticks: number, damage: number, which: Slot?): Object?
    local o = battle.spawn(doll.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        o.related1 = user
        local s = o.state :: State
        s.ticks = ticks
        if which then
            s.slot = which
            s.registers = true
        end
    end
    return o
end

return doll
