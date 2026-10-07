--!strict
-- Satelity's dimming controller (EXE5's effect object #0x3F, 0x080E4C54; EXE5's own, on the dimming chips' skeleton): the
-- usual dimming phases (@exelib/dimming), with an effect (0x080E4C98) in two parts. First SearchMan's scope
-- (chips/serchman/scope, as a sight, at the plain chip's speed) sweeps the enemy's area until its user locks it or
-- its time runs out, and gives the controller its panel. Then the user holds his animation 21 while the satellite
-- (./satellite) comes down three panels behind that panel and fires, its damage the chip's and the Atk+ bonus: 120
-- ticks, then the user stands again and the undim.

local dimming_chips = require("@exelib/dimming")
local satellite = require("./satellite")
local scope = require("../../chips/serchman/scope")

export type State = {
    -- The Atk+ / cross bonus (object +0x32).
    bonus: number,
    -- The scope's power-up (+5: B pressed with the lock), which nothing of Satelity's reads.
    powered: boolean,
}

local controller = {}

-- The effect's parts (0x080E4CAC), on the phase.
local AIM, FIRE = 0x0, 0x4
-- The user's animation while the satellite is at work.
local ANIM_CALL = 21
-- The satellite's parameters: it runs while the battle is dimmed (4), its beam's hit lasts 60 ticks.
local BEAM_TICKS = 0x3C
-- The panels the satellite hangs behind its target.
local BEHIND = 3
local FIRE_TICKS = 120

-- 0x080E4CB4: the scope out; once it has given a panel (the column it writes is never 0), the satellite.
local function aim(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.future_panel_x = 0
        scope.spawn(me, me.related1 :: Object, 6, true)
        return
    end
    if me.future_panel_x ~= 0 then
        me.phase, me.phase_init = FIRE, 0
    end
end

-- 0x080E4CE0: the user's pose and the satellite; 120 ticks, the user standing again, and the undim.
local function fire(me: Object, s: State)
    local user = me.related1 :: Object
    if me.phase_init == 0 then
        me.phase_init = 4
        user.anim = ANIM_CALL
        user:update_sprite_while_dimmed()
        local x = me.future_panel_x - BEHIND * me.facing
        satellite.spawn(me, x, me.future_panel_y, me.element, dimming_chips.damage_plus(me, s.bonus), true, BEAM_TICKS)
        me.timer = FIRE_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    user.anim = 0
    user:update_sprite_while_dimmed()
    dimming_chips.done(me)
end

-- 0x080E4C98.
local function effect(me: Object)
    local phase = me.phase
    if phase == AIM then
        aim(me)
    elseif phase == FIRE then
        fire(me, me.state :: State)
    else
        error(string.format("Satelity's controller: phase %#x of its effect reads past its table (0x080E4CAC)", phase))
    end
end

controller.kind = new.kind {
    id = "searchman/satelity/controller",
    pool = "effect",
    state = { bonus = "u16", powered = "bool" },
    -- 0x080E4C54.
    update = dimming_chips.phases { name = "Satelity", effect = effect },
}

-- 0x080E4D42: the controller for `user`, with the attack's element and damage word and the chip's bonus. (It takes
-- no panel: its own stays the spawner's zero until the scope gives it the target's.)
function controller.spawn(user: Object, element: number, damage: number, bonus: number): Object?
    local o = battle.spawn(controller.kind, Vec3.zero)
    if o then
        (o.state :: State).bonus = bonus
        o.element = element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
    end
    return o
end

return controller
