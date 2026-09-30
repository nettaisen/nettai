--!strict
-- The Mine chip's dimming controller (effect object #0x29, `sub_80E342C`):
-- the usual dimming phases (the dimming service), with an effect that lays
-- a mine (objects/land-mine) for the user's side with the damage plus the
-- bonus, and waits 121 ticks.
--
-- The chip's script (dimming chip subtype 11): `dimming_chip` is its
-- controller's spawner, `off_802CCB4[11]`.

local land_mine = require("../land-mine/land_mine")

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32); the bonus adds to the mine's damage.
    chip: number,
    bonus: number,
}

local mine = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x78

-- `sub_80E349E`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function mine.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("mine", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance = user.alliance
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = spec.chip, spec.bonus
    return o
end

-- `sub_80E3470`: the mine, then 121 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        local s = me.state :: State
        land_mine.spawn(me.alliance, int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus))
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E342C`: the dimming's phases around the effect.
function mine.update(me: Object)
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
            error(string.format("Mine's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return mine
