--!strict
-- The Invisibl and WhiCapsl chips' dimming controller (effect object #0x5D,
-- `sub_80E74D4`): the usual dimming phases (the dimming service), with an
-- effect that turns the user invisible, flashing for Param1-2 ticks
-- (`sub_8010474`), then waits 31 ticks. See docs/engine/chips.md §3.6.
--
-- The chips' script (both are dimming chip subtype 1): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[1]`.

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32).
    chip: number,
    bonus: number,
}

local invisible = {
    state = { chip = "u16", bonus = "u16" },
}

local SOUND = 0x93
local WAIT = 0x1E

-- `sub_80E7546`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function invisible.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("invisible", Vec3.zero, spec.params)
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

-- `sub_80E7518`: the user turns invisible, then 31 ticks pass.
local function effect(me: Object)
    if me.phase_init == 0 then
        local user = me.related1
        if not user then
            error("Invisibl's controller has no user (sub_8010474 writes through a null pointer)")
        end
        -- sub_8010474
        user:set_status_timer("flash", me:param(1) + me:param(2) * 0x100)
        user:set_status("invisible", true)
        battle.play_sound(SOUND)
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E74D4`: the dimming's phases around the effect.
function invisible.update(me: Object)
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
            error(string.format("Invisibl's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return invisible
