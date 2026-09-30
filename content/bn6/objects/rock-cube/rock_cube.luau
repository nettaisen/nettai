--!strict
-- The RockCube and IceCube chips' dimming controller (effect object #0x37,
-- `sub_80E4634`): the usual dimming phases (the dimming service), with an
-- effect that places a rock (objects/rock) on the panel in front of the
-- user, of the chip's variant (Param1: 1 a rock cube, 3 an ice block), and
-- waits 60 ticks.
--
-- The chips' script (both are dimming chip subtype 6): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[6]`.

local rock = require("../rock/rock")

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32).
    chip: number,
    bonus: number,
}

local rock_cube = {
    state = { chip = "u16", bonus = "u16" },
}

local PLACE_SOUND = 0x112
local WAIT = 0x3C

-- `sub_80E46B6`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function rock_cube.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("rock-cube", Vec3.zero, spec.params)
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

-- `sub_80E4678`: a rock in front of the user, then 60 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local params = { me:param(1), me:param(2), me:param(3), me:param(4) }
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        rock.spawn(me.panel_x + me.facing, me.panel_y, me.alliance, params, damage)
        battle.play_sound(PLACE_SOUND)
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E4634`: the dimming's phases around the effect.
function rock_cube.update(me: Object)
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
            error(string.format("RockCube's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return rock_cube
