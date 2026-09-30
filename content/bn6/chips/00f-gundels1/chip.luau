--!strict
-- GunDelSol1/2/3/EX: action 0x37 (`sub_80EDAE0`). See chips.md §4.
--
-- A 7-tick wind-up with the gun out, then one hit per tick for 60, 90 or
-- 120 ticks on the column two panels ahead (a 2x3 block for EX) under the
-- sun beam, then an 11-tick recovery. The hits are element 5: they drain
-- HP on the target's next update without flinching it or making it flash.
-- The damage is 2 per hit, 4 in the sun, whatever the chip data, Atk+ or
-- forms say. What differs between the chips (the gun, how long it fires,
-- the beam in and out of the sun) is each chip's own data
-- (`data.chips[id].gun_del_sol`).

local attachment = require("../../objects/attachment/attachment")
local sun_beam = require("../../objects/sun-beam/sun_beam")

export type State = {
    -- Ticks left in the current phase.
    timer: number,
}

local gun_del_sol = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local WIND_UP, FIRING, RECOVER = 0x0, 0x4, 0x8

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The gun's animation steps forward (out, firing, away). Without a gun
-- (its pool was full) there is nothing to step.
local function advance_gun(me: Object)
    local gun = me.overlay
    if gun then
        gun.anim += 1
    end
end

-- The chip's GunDelSol data (its gun, firing time and beam).
local function chip_data(me: Object): GunDelSol
    local chip = data.chips[me.chip]
    local d = chip and chip.gun_del_sol
    if not d then
        error(string.format("chip %#x runs GunDelSol without its data", me.chip))
    end
    return d
end

-- `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(0x0A)
        me:open_counter_window()
        attachment.spawn(me, { kind = chip_data(me).gun.id }, "overlay")
        battle.play_sound(0xF8)
        s.timer = 6
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return
    end
    local d = chip_data(me)
    s.timer = d.firing_ticks
    advance_gun(me)
    local look = if battle.navi(me.alliance).sun then d.beam_in_sun else d.beam
    me.related1 = sun_beam.spawn(me, look, Vec3.px(me.facing * 0x50, 0, 0), "related")
    set_phase(me, FIRING)
end

-- `sub_80EDBCC`: one hit this tick, until the timer runs out.
local function fire(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        local beam = me.related1
        if beam then
            sun_beam.stop(beam)
        end
        set_phase(me, RECOVER)
        return
    end
    battle.hitbox(me, {
        panel_x = me.panel_x + 2 * me.facing,
        panel_y = me.panel_y,
        element = 5,
        -- EX covers two columns.
        region = if me.variant < 3 then 0x04 else 0x11,
        hit_effect = 0xFF,
        target = 0x05,
        self_type = 0x2C,
        damage = if battle.navi(me.alliance).sun then 4 else 2,
    })
    -- (The game then reads the types of the panels hit and discards them.)
end

-- `sub_80EDC78`: put the gun away; 11 ticks later, back to idle. The gun
-- and the beam see their slots cleared and end themselves.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        advance_gun(me)
        s.timer = 10
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me:set_status("using_action", false)
    me:set_animation(0)
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

-- `sub_80EDAE0`: the phase, then the reactive-defense abort.
function gun_del_sol.update(me: Object, s: State)
    local step = me.step
    if step == WIND_UP then
        wind_up(me, s)
    elseif step == FIRING then
        fire(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("GunDelSol phase %#x reads past its phase table", step))
    end
    me:check_reactive_abort()
end

return gun_del_sol
