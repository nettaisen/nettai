--!strict
-- The TimeBom chips' dimming controller (effect object #0x27,
-- `sub_80E31D8`): the usual dimming phases (the dimming service), with an
-- effect that sets a countdown bomb (objects/countdown-bomb, of the chip's
-- variant: Param1 0 TimeBom1-3, 1 TimeBom+) on the first free enemy panel
-- in front of the user, and waits 31 ticks.
--
-- The chips' script (all are dimming chip subtype 10): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[10]`.

local countdown_bomb = require("../countdown-bomb/countdown_bomb")

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32); the bonus adds to the bomb's damage.
    chip: number,
    bonus: number,
}

local time_bom = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x1E

-- `off_80E3280`: where the bomb goes, by side: a solid enemy panel without
-- a body on it (side 0: flags 0x30, not 0x0F800000; side 1: 0x10, not
-- 0x0F800020).
local TARGET: { [number]: PanelCondition } = {
    [0] = { require = 0x30, forbid = 0x0F80_0000 },
    [1] = { require = 0x10, forbid = 0x0F80_0020 },
}

-- `sub_80E3242`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function time_bom.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("time-bom", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = spec.chip, spec.bonus
    return o
end

-- `sub_80E3264` (`object_getFirstPanelInDirectionFiltered`): from the
-- controller's panel forward, the first panel meeting the side's
-- condition, while the panels are on the field (nil: none).
local function target_column(me: Object): number?
    local cond = TARGET[me.alliance]
    local x, y = me.panel_x, me.panel_y
    while true do
        if field.check(x, y, cond.require, cond.forbid) then
            return x
        end
        x += me.facing
        if not field.valid(x, y) then
            return nil
        end
    end
end

-- `sub_80E3290`: the bomb, with the damage plus the bonus.
local function place(me: Object)
    local x = target_column(me)
    -- (The game tests the column number for zero: column 0 is off the
    -- field, so a found panel always places.)
    if x then
        local s = me.state :: State
        local params = { me:param(1), me:param(2), me:param(3), me:param(4) }
        local damage = int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus)
        countdown_bomb.spawn(x, me.panel_y, me.alliance, params, damage, s.bonus)
    end
end

-- `sub_80E321C`: the bomb, then 31 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        place(me)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E31D8`: the dimming's phases around the effect.
function time_bom.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("TimeBom's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return time_bom
