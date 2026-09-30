--!strict
-- SlashMan, the navi of the SlashMan chips (actor object #0x0D,
-- `sub_80B9F44`), brought by their navi chip controller. He appears and
-- sends sword waves (objects/slash-wave) down the columns: from the
-- farthest column with something of the other side's in it back toward
-- himself, a column every 15 ticks, one wave on each free panel of it
-- (the user's waves, hitting for the chip's Param1). Then he leaps to the
-- panel in front of the first enemy navi ahead and slashes it (region 4),
-- and leaves; the controller moves on. See docs/engine/chips.md §3.6.18.
--
-- The chips' script (navi chip subtype 4): `navi_chip` is his spawner,
-- `off_802CD5C[4]`.

local panels = require("../../lib/panels")
local slash_wave = require("../slash-wave/slash_wave")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local slash_man = {
    state = { controller = "object" },
}

local SPRITE = "08-03"

-- Panel flags he needs to stay (as one word).
local CAN_STAND = 0x10010

-- By side: a column worth sending waves to (`byte_80BA228`) and a panel a
-- wave goes on (`byte_80BA284`): require, forbid.
local TARGET_COLUMN = { [0] = { 0x0001_0000, 0x0F88_00A0 }, { 0x0001_0020, 0x0F88_0080 } }
local WAVE_PANEL = { [0] = { 0, 0x0F88_00A0 }, { 0x20, 0x0F88_0080 } }

-- What he leaps at (`off_80BA308`: an enemy navi's body), and the panel he
-- lands on (`dword_80BA314`: solid, unoccupied).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local LAND_REQUIRE, LAND_FORBID = 0x10, 0x0F88_0080

-- `sub_80BA1B0`: SlashMan for `user`, reporting to `controller`. (His
-- position is the spawner's registers, all of which his init overwrites.)
function slash_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("slash-man", Vec3.zero, spec.params)
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

-- `sub_80B9F68`: on his panel, on the ground.
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

-- Enter the phase (phase 4, a word store of the timers), or count down;
-- true once the timer is 0 or below (or, `past_zero`, below 0).
local function countdown(me: Object, ticks: number, past_zero: boolean): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer, me.timer2 = ticks, 0
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return if past_zero then left < 0 else left <= 0
end

-- `sub_80B9FD8`: he appears; 3 ticks; on, or straight to leaving if his
-- panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(0x94)
        me.visible = true
    end
    if not countdown(me, 3, false) then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then 4 else 0x1C)
end

-- `sub_80BA02C`: 10 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0xA, false) then
        me:set_action(8)
    end
end

-- `sub_80BA1D6`: the farthest column (from the other side's edge toward
-- him) with a panel worth a wave; 0 if none before his own.
local function first_column(me: Object): number
    local back = -me.facing
    local x = if me.alliance == 0 then 6 else 1
    local t = TARGET_COLUMN[me.alliance]
    while x ~= me.panel_x do
        if #panels.in_column(x, t[1], t[2]) ~= 0 then
            return x
        end
        x += back
        if x < 1 or x > 6 then
            return 0
        end
    end
    return 0
end

-- `sub_80BA054`: aim at the first column (Param2); 10 ticks. Nothing to
-- aim at: on to the leap.
local function aim(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local x = first_column(me)
        if x == 0 then
            me:set_action(0x10)
            return
        end
        me:set_param(2, x)
        me.anim = 0x14
        me.timer = 0xA
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0xC)
    end
end

-- `sub_80BA238`: a wave on each free panel of column x, the user's (their
-- damage the chip's Param1 with his damage's flag bits).
local function waves(me: Object, x: number)
    local user = me.related1 :: Object
    local w = WAVE_PANEL[me.alliance]
    local damage = bit32.bor(me:param(1), bit32.band(me.damage, 0xF000))
    for y = 1, 3 do
        if field.check(x, y, w[1], w[2]) then
            -- r3 (his damage's flag bits) becomes its Z.
            slash_wave.spawn(user, x, y, 0, { 1 }, damage, bit32.band(me.damage, 0xF000))
        end
    end
end

-- `sub_80BA090`: a column of waves every 15 ticks, back toward him, until
-- his own column or the field's edge.
local function slash_columns(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 0
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    waves(me, me:param(2))
    local x = me:param(2) - me.facing
    me:set_param(2, x)
    if x == me.panel_x or x < 1 or x > 6 then
        me:set_action(0x10)
        return
    end
    me.timer = 0xF
end

-- `sub_80BA0D8`: 60 ticks.
local function rest(me: Object)
    if countdown(me, 0x3C, false) then
        me:set_action(0x14)
    end
end

-- `sub_80BA294`: the panel in front of the first enemy navi ahead (in the
-- first of five columns with one), if he can land there, or it is his
-- own; else none.
local function landing(me: Object): (number?, number?)
    for i = 1, 5 do
        local x = me.panel_x + me.facing * i
        if not field.valid(x, 1) then
            return nil, nil
        end
        local found = panels.in_column(x, ENEMY_NAVI[me.alliance], 0)
        if #found ~= 0 then
            local fx, fy = panels.unpack(found[1])
            local lx = fx - me.facing
            if field.check(lx, fy, LAND_REQUIRE, LAND_FORBID) then
                return lx, fy
            end
            if lx == me.panel_x and fy == me.panel_y then
                return lx, fy
            end
            return nil, nil
        end
    end
    return nil, nil
end

-- `sub_80BA0FC`: the leap: out (anim 4), on the landing panel at 13 left
-- (anim 3); 16 ticks. Nowhere to land: he leaves.
local function leap(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local x, y = landing(me)
        if not x or x == 0 then
            me:set_action(0x1C)
            return
        end
        me.future_panel_x, me.future_panel_y = x, y :: number
        me.anim = 4
        me.timer = 0x10
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0xD then
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        me.anim = 3
        return
    end
    if left <= 0 then
        me:set_action(0x18)
    end
end

-- `sub_80BA31C`: the slash on the panel ahead (region 4, no hit spark,
-- running while dimmed), and its effect (0x39) there, 16 pixels up.
local function slash(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = 4,
        hit_effect = 0xFF,
        target = 0,
        self_type = 7,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), 0x39, bit32.bxor(me.alliance, me.flip))
end

-- `sub_80BA152`: the slash (sound 0x158); 30 ticks.
local function strike(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x10
        battle.play_sound(0x158)
        slash(me)
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0x1C)
    end
end

-- `sub_80BA186`: 4 ticks (to -1), then he goes and the controller moves
-- on.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim = 4
    end
    if not countdown(me, 3, true) then
        return
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = {
    [0] = appear,
    [4] = stand,
    [8] = aim,
    [0xC] = slash_columns,
    [0x10] = rest,
    [0x14] = leap,
    [0x18] = strike,
    [0x1C] = leave,
}

-- `sub_80B9F44`.
function slash_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("SlashMan action %#x reads past his table (off_80B9FB8)", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return slash_man
