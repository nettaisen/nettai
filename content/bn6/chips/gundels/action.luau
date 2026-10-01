--!strict
-- GunDelSol's action (the original's 0x37, `sub_80EDAE0`; chips.md §4): a
-- 7-tick wind-up with the gun out, then one hit per tick for the chip's
-- firing time on the column two panels ahead (a column and the one past
-- it, wide) under the sun beam (./beam), then an 11-tick recovery. The
-- hits are element 5: they drain HP on the target's next update without
-- flinching it or making it flash. The damage is 2 per hit, 4 in the sun,
-- whatever the chip data, Atk+ or forms say. What differs between the
-- chips (the gun, how long it fires, the beam in and out of the sun, the
-- width) is each chip's (`action { ... }`, `GunDelSolSpec`).

local attachment = require("../../objects/attachment/attachment")
local beam = require("./beam")
local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local action = {}

export type State = {
    -- Ticks left in the current phase.
    timer: number,
}

-- One state for every GunDelSol: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local WIND_UP, FIRING, RECOVER = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0A
local SOUND = asset.sound("gundels1")
local WIND_UP_TICKS, RECOVER_TICKS = 6, 10
-- The beam, 80 pixels ahead of the navi.
local BEAM_AHEAD = 0x50
-- The hits: element 5, two panels ahead, a drain, 2 damage (4 in the
-- sun).
local ELEMENT, AHEAD = 5, 2
local HIT = { is = collision.drain, hits = collision.hits_navis }
local DAMAGE, SUN_DAMAGE = 2, 4

-- The gun (the original's attachment rows 7 to 0xA, by the chip), in its
-- palette.
function action.gun(palette: number): AttachmentLook
    return attachment.look { sprite = asset.sprite("gun-del-sol"), palette = palette, attach_point = 0x0D }
end

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

-- A chip's GunDelSol.
function action.action(spec: GunDelSolSpec): Action
    local region = if spec.wide then regions.wide_long else regions.wide

    -- `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
    local function wind_up(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me:set_animation(ANIM_RAISE)
            me:open_counter_window()
            attachment.attach(me, spec.gun, "overlay")
            battle.play_sound(SOUND)
            s.timer = WIND_UP_TICKS
            me.step_init = 4
            return
        end
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        s.timer = spec.firing_ticks
        advance_gun(me)
        local look = if battle.navi(me.alliance).sun then spec.beam_in_sun else spec.beam
        me.related1 = beam.spawn(me, look, Vec3.px(me.facing * BEAM_AHEAD, 0, 0), "related")
        set_phase(me, FIRING)
    end

    -- `sub_80EDBCC`: one hit this tick, until the timer runs out.
    local function fire(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            local b = me.related1
            if b then
                beam.stop(b)
            end
            set_phase(me, RECOVER)
            return
        end
        battle.hitbox(me, {
            panel_x = me.panel_x + AHEAD * me.facing,
            panel_y = me.panel_y,
            element = ELEMENT,
            region = region,
            -- None.
            hit_effect = 0xFF,
            target = HIT.hits,
            self_type = HIT.is,
            damage = if battle.navi(me.alliance).sun then SUN_DAMAGE else DAMAGE,
        })
        -- (The game then reads the types of the panels hit and discards
        -- them.)
    end

    -- `sub_80EDC78`: put the gun away; 11 ticks later, back to idle. The
    -- gun and the beam see their slots cleared and end themselves.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            advance_gun(me)
            s.timer = RECOVER_TICKS
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
    local function update(me: Object, s: State)
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

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
