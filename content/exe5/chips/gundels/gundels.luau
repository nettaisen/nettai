--!strict
-- EXE5's GunDelSol (action 0x2F, 0x080EDCB8; EXE6's action 0x37, `sub_80EDAE0`): GunDelS1 to 3's and GunDelEX's. A
-- 7-tick wind-up with the gun out, then one hit per tick on the column two panels ahead (and the one past it,
-- GunDelEX's) under the sun beam (@exelib/gundels/beam, EXE5's hum), for the chip's firing time while its player
-- holds A, then an 11-tick recovery. The hits are element 5: they drain HP without flinching; 2 damage a hit, 4 in
-- the sun. EXE6's code but:
--
-- - a player's beam lasts only while A is held (0x080EDD30, 0x080EDDCE): at the wind-up's end and each firing
--   tick, a player (not AI index 0x17) without the no-charge state who has let go of A stops firing (unless the
--   battle gives each player a gauge, the operation battle); the beam also stops when the battle is over;
-- - the hits are EXE5's collision row 0x17 (EXE6's drain, 0x2C);
-- - the gun is the attachment table's rows 0x13 to 0x16 (EXE6's 7 to 0xA), the same look;
-- - the firing ticks are EXE5's (0x080EDDC8: 60, 90, 120, 120), and nothing is left in the attack's count;
-- - the reactive-defense abort takes requests 0x200 and 0x400 (EXE6's also 0x8000);
-- - the recovery steps the gun's animation without a null check (EXE6 checks; a missing gun's write goes nowhere).

local attachment = require("../../objects/attachment")
local beam = require("@exelib/gundels/beam")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local operation_battle = require("../../lib/operation_battle")

local gundels = {}

export type State = {
    -- Ticks left in the current phase.
    timer: number,
}

-- One state for every GunDelSol.
gundels.STATE = { timer = "u16" }

-- What a chip's GunDelSol holds and fires: its gun, how long it fires at most, the beam in the shade and in the
-- sun, and whether its hits cover the column past the target's too (GunDelEX's).
export type Spec = {
    id: string?,
    gun: AttachmentLook,
    firing_ticks: number,
    beam: SunBeamLook,
    beam_in_sun: SunBeamLook,
    wide: boolean?,
}

gundels.beam_kind = beam.make_kind("gundels/beam", asset.sound("sun-beam"))

-- The beams' looks (0x080EDDB8): GunDelSol's, and GunDelEX's wider one, in the shade and in the sun.
local PLAIN, WIDE = asset.sprite("sun-beam"), asset.sprite("sun-beam-ex")
gundels.beams = {
    plain = { sprite = PLAIN, palette = 0 },
    plain_in_sun = { sprite = PLAIN, palette = 2 },
    wide = { sprite = WIDE, palette = 0 },
    wide_in_sun = { sprite = WIDE, palette = 3 },
}

-- The phases, on the attack's step.
local WIND_UP, FIRING, RECOVER = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0A
local SOUND = asset.sound("gundels1")
local WIND_UP_TICKS, RECOVER_TICKS = 6, 10
-- The beam, 80 pixels ahead of the navi.
local BEAM_AHEAD = 0x50
-- The hits: element 5, two panels ahead, EXE5's row 0x17, 2 damage (4 in the sun).
local ELEMENT, AHEAD = 5, 2
local HIT = { is = collision.thrown_piercing_break, hits = collision.hits_navis }
local DAMAGE, SUN_DAMAGE = 2, 4
-- The AI index whose player fires without holding A.
local HOLDLESS_AI = 0x17

-- The gun (the attachment table's rows 0x13 to 0x16, by the chip), in its palette.
function gundels.gun(palette: number): AttachmentLook
    return attachment.look { sprite = asset.sprite("gun-del-sol"), palette = palette, attach_point = 0x0D }
end

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The gun's animation steps forward (out, firing, away).
local function advance_gun(me: Object)
    local gun = me.overlay
    if gun then
        gun.anim += 1
    end
end

-- 0x080EDD30 / 0x080EDDCE: a player who must hold A to keep firing has let go of it.
local function let_go(me: Object): boolean
    if operation_battle() then
        return false
    end
    if me.actor_type ~= "player" or me.ai_index == HOLDLESS_AI then
        return false
    end
    if me:navi_state("no_charge") then
        return false
    end
    return not me:held("a")
end

-- A chip's GunDelSol.
function gundels.action(spec: Spec): Action
    local region = if spec.wide then regions.wide_long else regions.wide

    -- 0x080EDCEC (`sub_80EDB14`): the gun comes out; 6 ticks later the beam lights up, if A is still held.
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
        if let_go(me) then
            me.step, me.step_init = RECOVER, 0
            return
        end
        s.timer = spec.firing_ticks
        advance_gun(me)
        local look = if battle.navi(me.alliance).sun then spec.beam_in_sun else spec.beam
        me.related1 = beam.spawn(gundels.beam_kind, me, look, Vec3.px(me.facing * BEAM_AHEAD, 0, 0), "related")
        set_phase(me, FIRING)
    end

    -- 0x080EDDCC (`sub_80EDBCC`): one hit this tick, until A is let go, the battle is over or the timer runs out.
    local function fire(me: Object, s: State)
        local stop = let_go(me) or battle.over()
        if not stop then
            local t = s.timer - 1
            s.timer = t
            stop = t < 0
        end
        if stop then
            local b = me.related1
            if b then
                beam.stop(b)
            end
            me.step, me.step_init = RECOVER, 0
            return
        end
        battle.hitbox(me, {
            panel_x = me.panel_x + AHEAD * me.facing,
            panel_y = me.panel_y,
            element = ELEMENT,
            region = region,
            -- None.
            hit_effect = nil,
            target = HIT.hits,
            self_type = HIT.is,
            damage = if battle.navi(me.alliance).sun then SUN_DAMAGE else DAMAGE,
        })
        -- (The game then reads the types of the panels hit and discards them.)
    end

    -- 0x080EDEA4 (`sub_80EDC78`): put the gun away; 11 ticks later, back to idle. The gun and the beam see their
    -- slots cleared and end themselves.
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

    -- 0x080EDCB8: the phase, then the reactive-defense abort (requests 0x200, 0x400).
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
        if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
            me:check_reactive_abort()
        end
    end

    return new.action { id = spec.id, state = gundels.STATE, args = spec, update = update }
end

return gundels
