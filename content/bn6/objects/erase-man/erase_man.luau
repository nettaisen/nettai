--!strict
-- EraseMan, the navi of the EraseMan chips (actor object #0x15,
-- `sub_80BB608`), brought by their navi chip controller. He appears and
-- aims a line of marks (objects/erase-mark) up-forward, forward or
-- down-forward from his row, switching every Param1 ticks, until his user
-- presses A (or 360 ticks pass); then he slashes along the line
-- (objects/erase-beam), and leaves. See docs/engine/chips.md §3.6.10.
--
-- The chips' script (navi chip subtype 5): `navi_chip` is his spawner,
-- `off_802CD5C[5]`.

local erase_mark = require("../erase-mark/erase_mark")
local erase_beam = require("../erase-beam/erase_beam")

export type State = {
    -- Where in his row's cycle the aim is (ExtraVars[0]).
    cycle: number,
    -- The aim: 0 up-forward, 1 forward, 2 down-forward (ExtraVars[1]).
    aim: number,
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local erase_man = {
    state = { cycle = "u8", aim = "u8", controller = "object" },
}

local SPRITE = "08-04"

-- The aims each row cycles through, five bytes a row, 0xFF ending a row
-- early (`byte_80BB88C`).
local END = 0xFF
local AIMS_BY_ROW = { [0] = 1, 2, END, END, END, 0, 1, 2, 1, END, 0, 1, END, END, END, 0 }

-- Each aim's step, forward and down (`byte_80BB860`).
local STEPS = { [0] = { 1, -1 }, { 1, 0 }, { 1, 1 } }

-- How long he aims at most, and the slash's segments last.
local AIM_TICKS, SLASH_TICKS = 0x168, 0x3C

-- Panel flags he needs to stay (as one word).
local CAN_STAND = 0x10010

-- `sub_80BB7F6`: EraseMan for `user`, reporting to `controller`.
function erase_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("erase-man", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80BB62C`: on his panel, on the ground.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- The entry of most of his actions: on phase 0 go to phase 4 with an
-- animation and a timer (Timer2 cleared), and say so.
local function enter(me: Object, anim: number, ticks: number): boolean
    if me.phase ~= 0 then
        return false
    end
    me.phase = 4
    me.anim = anim
    me.timer = ticks
    me.timer2 = 0
    return true
end

-- Count the timer down; true at 0 or below.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- `sub_80BB866`: the next aim of his row's cycle.
local function next_aim(me: Object)
    local s = me.state :: State
    local row = (me.panel_y - 1) * 5
    local i = s.cycle + 1
    local aim = AIMS_BY_ROW[row + i]
    if aim == nil then
        error(string.format("EraseMan in row %d reads past his aims", me.panel_y))
    end
    if aim == END then
        aim, i = AIMS_BY_ROW[row], 0
    end
    s.aim, s.cycle = aim, i
end

-- The panels along his aim, up to five, while on the field (the walk of
-- `sub_80BB81C` and `sub_80BB8B0`).
local function each_panel(me: Object, f: (x: number, y: number) -> ())
    local step = STEPS[(me.state :: State).aim]
    for i = 1, 5 do
        local x = me.panel_x + step[1] * me.facing * i
        local y = me.panel_y + step[2] * i
        if not field.valid(x, y) then
            return
        end
        f(x, y)
    end
end

-- `sub_80BB81C` and its sound: mark the aim's panels.
local function mark(me: Object)
    local ticks = me:param(1)
    each_panel(me, function(x, y)
        erase_mark.spawn(me, x, y, ticks)
    end)
    battle.play_sound(0x10E)
end

-- `sub_80BB694`: he appears; 3 ticks; on to standing, or straight to
-- leaving if his panel can't hold him.
local function appear(me: Object)
    if enter(me, 3, 3) then
        battle.play_sound(0x94)
        me.visible = true
        return
    end
    if not ran_out(me) then
        return
    end
    me:set_action(if field.check(me.panel_x, me.panel_y, CAN_STAND, 0) then 4 else 0x14)
end

-- `sub_80BB6E8`: 30 ticks standing.
local function stand(me: Object)
    if enter(me, 0, 0x1E) then
        return
    end
    if ran_out(me) then
        me:set_action(8)
    end
end

-- `sub_80BB710`: aim, switching every Param1 ticks, until the user
-- presses A while dimmed (`sub_80BB89C`), or for 360 ticks.
local function aim(me: Object)
    if me.phase == 0 then
        me.phase = 4;
        (me.state :: State).cycle = 3
        next_aim(me)
        mark(me)
        me.timer = AIM_TICKS
        me.timer2 = me:param(1)
        return
    end
    local user = me.related1 :: Object
    if user:dimmed_pressed("a") or ran_out(me) then
        me:set_action(0xC)
        return
    end
    local left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    next_aim(me)
    mark(me)
    me.timer2 = me:param(1)
end

-- `sub_80BB772`: 10 ticks with the scythe raised.
local function raise(me: Object)
    if enter(me, 0x11, 0xA) then
        return
    end
    if ran_out(me) then
        me:set_action(0x10)
    end
end

-- `sub_80BB79A`: the slash along the aim (`sub_80BB8B0`: a beam segment
-- on each panel, lasting 60 ticks); 60 ticks.
local function slash(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x12
        local s = me.state :: State
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        each_panel(me, function(x, y)
            erase_beam.spawn(me, x, y, me.element, { s.aim, SLASH_TICKS, 0 }, damage)
        end)
        battle.play_sound(0xBA)
        me.timer = SLASH_TICKS
        return
    end
    if ran_out(me) then
        me:set_action(0x14)
    end
end

-- `sub_80BB7CC`: 3 ticks, then he goes and the controller moves on.
local function leave(me: Object)
    if enter(me, 4, 3) then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = aim, [0xC] = raise, [0x10] = slash, [0x14] = leave }

-- `sub_80BB608`.
function erase_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("EraseMan action %#x reads past his table", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return erase_man
