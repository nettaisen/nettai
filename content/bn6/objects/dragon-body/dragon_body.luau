--!strict
-- A dragon's body segment (attack object #0xC8, `sub_80DE13C`). Its head
-- (objects/dragon-head) spawns four on its own panel; each waits its
-- delay (3, 6, 9 or 12 ticks) hidden, then follows the head's path: down
-- the column, a panel forward in a dip, and back up out of sight. It
-- doesn't hit anything. See lib/dragon.luau.

local dragon = require("../../lib/dragon")

local dragon_body = {}

export type State = {
    -- Ticks left to wait, ticks into the swim, or left in the climb out
    -- (EV+0).
    count: number,
    -- Down or up its column (EV+2).
    leg: number,
    -- Ticks a panel takes (EV+3), and the dip's angle step and angle
    -- (EV+4, EV+5).
    ticks_per_panel: number,
    angle_step: number,
    angle: number,
    -- Its height where the dip starts (EV+8).
    base_y: number,
}

dragon_body.state = {
    count = "u8",
    leg = "u8",
    ticks_per_panel = "u8",
    angle_step = "u8",
    angle = "u8",
    base_y = "i32",
}

local BODY_ANIM = 7
-- Its actions: wait, dive or climb, swim, leave.
local WAIT, COLUMN, SWIM, LEAVE = 0x0, 0x4, 0x8, 0xC

-- `sub_80DE2F0`: segment `i` of `head`'s dragon, `delay` ticks behind. Its
-- position is the spawner's registers (the head's row, i and the delay)
-- until its init places it; the delay stays in Z's fraction.
function dragon_body.spawn(head: Object, i: number, delay: number): Object?
    local o = battle.spawn_kind("dragon-body", Vec3.new(head.panel_y, i, delay), { head:param(1) })
    if o then
        o.panel_x, o.panel_y = head.panel_x, head.panel_y
        local s = o.state :: State
        s.count = delay
        o.alliance, o.flip = head.alliance, head.flip
    end
    return o
end

-- `sub_80DE1F4`: hidden until its delay is up; then it splashes in.
local function wait(me: Object)
    local s = me.state :: State
    local n = s.count - 1
    s.count = n
    if n ~= 0 then
        return
    end
    me.visible = true
    dragon.splash(me, 1)
    me.action = COLUMN
    me.phase = 0
end

-- `sub_80DE21A`: down (or up) its column; at the bottom row, swim; at the
-- row above the field, leave.
local function column(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local s = me.state :: State
    if me.panel_y ~= bit32.bxor(s.leg, 1) * 3 then
        return
    end
    if s.leg == dragon.UP then
        s.count = dragon.exit_ticks(me, s.leg)
        me.action = LEAVE
        me.phase = 0
    else
        s.angle = 0
        s.base_y = me.pos.y
        me.action = SWIM
    end
end

-- `sub_80DE266`: a panel forward in a dip; then turn up.
local function swim(me: Object)
    local s = me.state :: State
    local n = s.count + 1
    s.count = n
    local p, v = me.pos, me.vel
    if n == s.ticks_per_panel then
        s.leg = bit32.bxor(s.leg, 1)
        local cx = field.center(me.panel_x, me.panel_y)
        me.vel = Vec3.new(v.x, -v.y, v.z)
        me.pos = Vec3.new(cx, p.y, p.z)
        s.count = 0
        me.action = COLUMN
    else
        me.pos = Vec3.new(p.x + v.x, p.y, p.z)
        local angle = s.angle + s.angle_step
        s.angle = angle
        dragon.dip(me, angle, s.base_y, s.leg)
        me:set_panel_from_coordinates()
    end
end

-- `sub_80DE2B0`: climb out of sight, then splash and go.
local function leave(me: Object)
    local s = me.state :: State
    if s.count ~= 0 then
        s.count -= 1
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x, p.y + v.y, p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    if me.phase == 0 then
        me.phase = 1
        dragon.splash(me, s.leg)
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80DE1B8`: follow the path, but not while dimmed; gone when the
-- battle is.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    local a = me.action
    if a == WAIT then
        wait(me)
    elseif a == COLUMN then
        column(me)
    elseif a == SWIM then
        swim(me)
    elseif a == LEAVE then
        leave(me)
    else
        error(string.format("dragon body action %#x reads past its table", a))
    end
end

-- `sub_80DE168`: hidden on the head's panel.
local function init(me: Object)
    dragon.place(me, BODY_ANIM)
    me.visible = false
    -- sub_80DE30E
    local s = me.state :: State
    local per_panel, step = dragon.start(me)
    s.ticks_per_panel = per_panel
    if per_panel ~= 0 then
        s.angle_step = step
    end
    s.leg = dragon.DOWN
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80DE13C`.
function dragon_body.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite_while_paused()
end

return dragon_body
