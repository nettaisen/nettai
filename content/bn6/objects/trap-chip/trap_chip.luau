--!strict
-- The trap chips' dimming controller (effect object #0x2A, `sub_80E34C0`):
-- AntiNavi, AntiDmg, AntiSwrd, AntiRecv, ElemTrap and BodyGrd. The usual
-- dimming phases (the dimming service), with the telop hidden, and an
-- effect that makes the chip its user's side's defensive chip (the linked
-- record: the damage intake springs the trap), with its object for the
-- kinds that have one (ElemTrap: objects/elem-trap), then waits 61 ticks.
-- See docs/engine/chips.md §3.6.9.
--
-- The chips' script (all are dimming chip subtype 20): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[20]`.

local elem_trap = require("../elem-trap/elem_trap")

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32), kept for the counterattack.
    chip: number,
    bonus: number,
}

local trap_chip = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x3C

-- `sub_80E353E`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function trap_chip.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("trap-chip", Vec3.zero, spec.params)
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

-- `sub_80E3560`: the trap's object, by Param1 (`off_80E3574`): ElemTrap's
-- (0, `sub_80E358C`), none for the rest (1 to 5, `sub_80E359E`).
local function trap_object(me: Object, user: Object): Object?
    local kind = me:param(1)
    if kind == 0 then
        local s = me.state :: State
        return elem_trap.spawn(me, user, s.chip, s.bonus)
    elseif kind <= 5 then
        return nil
    end
    error(string.format("trap chip Param1 %d reads past sub_80E3560's table", kind))
end

-- `sub_80E3504`: the side's previous defensive chip goes; this one takes
-- its place (`sub_802CE8A`, on its user's side); 61 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        battle.clear_linked(me.alliance)
        local user = me.related1
        if not user then
            error("the trap chip's controller has no user (sub_802CE8A reads through a null pointer)")
        end
        local object = trap_object(me, user)
        local s = me.state :: State
        battle.set_linked(user.alliance, {
            chip = s.chip,
            bonus = s.bonus,
            damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
            owner = user,
            object = object,
        })
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E34C0`: the dimming's phases around the effect.
function trap_chip.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_hidden_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("the trap chips' action %#x reads past their table", a))
        end
    else
        dimming.finish(me)
    end
end

return trap_chip
