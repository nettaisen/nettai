--!strict
-- BN6's form change (`sub_8014A38`): while the battle is paused at a turn's
-- start, the navi changes into the form its side asked for (the turn's
-- transform record, `navi:form_change_target()`). The turn-start sequencer
-- (the framework's, BN5 has it too) waits for it. Every BN6 form names this
-- action as its `change`.
--
-- Five sequences, the routine's tables, each of four steps (the attack's
-- step 0, 4, 8, 0xC):
--
-- | Target                 | From          | Table        |                                                    |
-- |------------------------|---------------|--------------|----------------------------------------------------|
-- | a Cross                | any           | `off_8014AB4` | the navi's image merges with MegaMan              |
-- | Beast Out              | any           | `off_8014AC8` |                                                    |
-- | a Cross in Beast Out   | no Beast form | `off_8014ADC` | a Cross Beast, Beast Out's way                     |
-- | a Cross in Beast Out   | a Beast form  | `off_8014B04` | a Beast's Cross: the Cross navi merges with the beast |
-- | Beast Over             | any           | `off_8014AF0` |                                                    |

local effects = require("../../lib/effects")
local merge = require("./merge")

local change = {}

export type State = {
    -- AIAttackVars+0x10: ticks left in the current step.
    timer: number,
}

export type Sequence = "cross" | "beast_out" | "cross_beast" | "beast_cross" | "beast_over"

-- The forms system's state (system.luau): what a change notes of the round.
type Record = { beast_out_used: boolean, crossed: boolean }

-- The sounds it plays.
local FORM_CHANGE = asset.sound("form-change")
local GREGAR_ROAR = asset.sound("gregar-roar")
local FALZAR_ROAR = asset.sound("falzar-roar")
local BEAST_OVER_RUMBLE = asset.sound("beast-over")
local BEAST_OUT = asset.sound("beast-out")
local CROSS_CHANGE = asset.sound("cross-change")
local CROSS_CHANGE_CHIME = asset.sound("log-in")

-- The steps, in the attack's step.
local PREPARE = 0
local VANISH = 4
local EMERGE = 8
local SETTLE = 0xC

-- Beast Out, with or without a Cross, or Beast Over.
local function is_beast(kind: string): boolean
    return kind == "beast" or kind == "cross_beast" or kind == "beast_over"
end

-- Which table a change takes, by the target's kind and the current form's.
local function sequence(target: string, current: string): Sequence
    if target == "beast_over" then
        return "beast_over"
    elseif target == "cross_beast" then
        return if is_beast(current) then "beast_cross" else "cross_beast"
    elseif target == "beast" then
        return "beast_out"
    else
        return "cross"
    end
end

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- Count the timer down; true once it had run out (it wraps).
local function timer_ran_out(s: State): boolean
    local old = s.timer
    s.timer = old - 1
    return old == 0
end

-- Count the timer down; true while it is still above 0.
local function timer_running(s: State): boolean
    s.timer -= 1
    return int.i16(s.timer) > 0
end

-- The navi's overlay (related2), if it has one.
local function overlay(me: Object): Object?
    return me.related2
end

-- Step 0: `sub_8014D08` (Beast Out), `sub_8014F40` (Cross Beast),
-- `sub_801516C` (Beast Over): onto the destination panel, facing the
-- default way, charge, aura and statuses dropped, links cut; the pose.
-- `sub_8014B18` (Cross) and `sub_80153EC` (a Beast's Cross): the sprite
-- holds still first, and the pose and links stay.
local function prepare(me: Object, s: State, seq: Sequence, current: FormDef)
    local crossing = seq == "cross" or seq == "beast_cross"
    if crossing then
        me:set_navi_state("form_change_sprite_held", true)
    end
    me:snap_to_future_panel()
    me:clear_invulnerable()
    me:face_default()
    me:reset_charge()
    me:end_full_synchro_aura()
    if not crossing then
        me.related1 = nil
        me.overlay = nil
    end
    me:drop_statuses()
    if seq == "cross" then
        s.timer = 6
        -- A GroundCross in its animation 0x16 (the form's
        -- `cross_release_anim`) lets go of it.
        if current.cross_release_anim ~= nil and current.cross_release_anim == me.anim then
            me.anim = 0
            me.related1 = nil
            me.overlay = nil
            me:set_navi_state("form_change_sprite_held", false)
            me:overlay_stepping("keep")
        end
    elseif seq == "beast_cross" then
        me:overlay_stepping("keep")
        s.timer = 6
    else
        me.anim = 0x11
        me:overlay_stepping("keep")
        s.timer = 6
    end
    set_step(me, VANISH)
end

-- Beast Over's two effects (`sub_80151D4`): its beast's and the blast
-- (palette by the beast), 32 pixels up. The second only after the first,
-- and the rest only after both.
local function beast_over_effects(pos: Vec3, alliance: number, target: FormDef): Object?
    local which: number
    if target.game == "gregar" then
        which = 0
    elseif target.game == "falzar" then
        which = 1
    else
        error(string.format("form %s has no Beast Over effect (sub_80151D4)", target.id))
    end
    local at = pos + Vec3.new(0, 0, 0x20_0000)
    local first = battle.effect(at, if which == 0 then effects.beast_over_gregar else effects.beast_over_falzar, alliance, 0, 0)
    if not first then
        return nil
    end
    first.timer = 0x36
    first.run_while_paused = true
    battle.play_sound(BEAST_OVER_RUMBLE)
    local second = battle.effect(at, effects.beast_over_blast, alliance, 2 * which, 0)
    if not second then
        return nil
    end
    second.timer = 0x45
    second.run_while_paused = true
    return second
end

-- Step 4 of Beast Out (`sub_8014D70`), a Cross Beast (`sub_8014FA8`) and
-- Beast Over (`sub_80151D4`): after 7 ticks the navi vanishes into the
-- beast effect (moved far off the field), which lasts 54 ticks.
local function vanish(me: Object, s: State, seq: Sequence, target: FormDef)
    if me.step_init == 0 then
        if not timer_ran_out(s) then
            return
        end
        me:set_navi_state("form_change_sprite_held", true)
        local pos, alliance = me.pos, me.alliance
        local spawned: Object?
        if seq == "beast_over" then
            spawned = beast_over_effects(pos, alliance, target)
        else
            battle.play_sound(FORM_CHANGE)
            spawned = battle.effect(pos, effects.form_change, alliance, 0, 0)
            if spawned then
                spawned.timer = 0x36
                spawned.run_while_paused = true
            end
        end
        if spawned then
            me.pos = me.pos + Vec3.new(0, 0xC0_0000, 0)
            -- `sub_800AB2E`
            local record = system.state() :: Record
            record.beast_out_used = true
            -- The roar is Gregar's for the Gregar beasts and Falzar's
            -- otherwise; the cameras shake at magnitude 2 (`sub_80302B6`),
            -- 60 ticks (75 for Beast Over).
            battle.play_sound(if target.game == "gregar" then GREGAR_ROAR else FALZAR_ROAR)
            battle.shake_camera_secondary(2, if seq == "beast_over" then 0x4B else 0x3C)
        end
        s.timer = 0x36
        if seq == "beast_over" then
            battle.burst(me)
        end
        me.step_init = 4
    end
    -- Beast Over's rumbles. (The game compares the timer as a word, whose
    -- upper half is the attack's count, which none of its steps writes:
    -- what an earlier action left there, 2 after a buster shot, keeps them
    -- quiet.)
    local word = s.timer + me.attack_count * 0x10000
    if seq == "beast_over" and (word == 0x35 or word == 0x25) then
        battle.play_sound(BEAST_OVER_RUMBLE)
    end
    if not timer_ran_out(s) then
        return
    end
    set_step(me, EMERGE)
end

-- Step 4 of a Cross (`sub_8014B98`) and a Beast's Cross (`sub_801544C`):
-- after 7 ticks the Cross navi's image appears above MegaMan (actor
-- object #0x1B) and merges with him; 49 ticks later he changes. The
-- attack marker counts the ticks MegaMan (and in a Beast's Cross, his
-- overlay) flashes white.
local function merging(me: Object, s: State, seq: Sequence, target: FormDef)
    if me.step_init == 0 then
        if not timer_ran_out(s) then
            return
        end
        -- The navi the Cross is made with.
        local navi = target.cross_of
        if navi == nil then
            error(string.format("form %s is a Cross of no navi (sub_8014B98)", target.id))
        end
        merge.spawn(me, navi, 0x14)
        s.timer = 0x30
        me.marker = 0
        me.step_init = 4
    end
    local next = me.marker + 1
    if next <= 6 then
        me.marker = next
        me.sprite.white = true
        local o = overlay(me)
        if seq == "beast_cross" and o then
            o.sprite.white = true
        end
    end
    local o = overlay(me)
    if seq == "beast_cross" and next == 7 and o then
        -- sprite_clearFinalPalette
        o.sprite.white = false
    end
    if not timer_ran_out(s) then
        return
    end
    s.timer = 6
    set_step(me, EMERGE)
    me.marker = 0
end

-- Step 8 (`sub_8014E08`, `sub_8015040`, `sub_80152C8`, `sub_8014BEE`,
-- `sub_80154C8`): MegaMan in the new form (sprite, form, name, overlay);
-- 10 ticks later the form's status set-up.
local function emerge(me: Object, s: State, seq: Sequence, target: FormDef)
    -- sprite_forceWhitePalette every tick.
    me.sprite.white = true
    local side = me.alliance
    if me.step_init == 0 then
        if seq == "cross" then
            me.related1 = nil
            me.overlay = nil
        elseif seq == "beast_cross" then
            local o = overlay(me)
            if o then
                o.run_while_paused = true
                o.run_while_dimmed = true
            end
        end
        local stats = battle.navi(side)
        me:take_off_form_overlay(stats.form)
        if seq == "beast_cross" then
            me.related1 = nil
            me.overlay = nil
        end
        me:load_form_sprite(target)
        me:set_coordinates_from_panel()
        s.timer = 10
        if seq == "beast_over" then
            me:end_statuses()
        end
        stats.form = target
        if seq == "beast_out" or seq == "cross_beast" or seq == "beast_over" then
            battle.palette_flash(0, 14, true, true)
            battle.play_sound(BEAST_OUT)
        end
        -- `sub_8015B22`: the form's NameID (the navi's own, for a form
        -- without one).
        me.identity = target.identity or stats.navi.identity
        me:put_on_form_overlay(target)
        local o = overlay(me)
        if o then
            if seq == "beast_out" or seq == "beast_over" then
                -- Its Param3 = 0.
                me:overlay_stepping("normal")
            elseif seq == "cross_beast" or seq == "beast_cross" then
                -- Its animation 0 and Param3 0 (and in a Cross Beast,
                -- flags 0x14).
                o.anim = 0
                me:overlay_stepping("normal")
                if seq == "cross_beast" then
                    o.run_while_paused = true
                    o.run_while_dimmed = true
                end
            end
        end
        if seq == "cross" or seq == "beast_cross" then
            battle.play_sound(CROSS_CHANGE)
            battle.play_sound(CROSS_CHANGE_CHIME)
        end
        me.step_init = 4
    end
    if timer_running(s) then
        return
    end
    local full_synchro = battle.emotion(side) == "full_synchro"
    me:reset_status()
    me:end_anger()
    if seq == "beast_out" then
        if full_synchro then
            battle.set_mood(side, 0xFF)
        end
    else
        battle.set_mood(side, 0x80)
    end
    if seq ~= "beast_over" then
        me:clear_invulnerable()
    end
    local o = overlay(me)
    if (seq == "beast_out" or seq == "beast_over" or seq == "beast_cross") and o then
        -- sprite_clearFinalPalette
        o.sprite.white = false
    end
    set_step(me, SETTLE)
end

-- The end of a form change: the flags come off (and battle flag 0x20,
-- which nothing sets), the reactive-defense requests are dropped, and the
-- navi idles.
local function finish(me: Object)
    me:set_navi_state("form_change", false)
    me:set_navi_state("form_change_sprite_held", false)
    me:set_request("weakness_hit", false)
    me:set_request("body_guard_triggered", false)
    me:set_request("anti_sword_triggered", false)
    me:set_request("anti_damage_triggered", false)
    me:exit_attack()
end

-- Step 0xC (`sub_8014F04`, `sub_8015128`, `sub_80153B0`, `sub_8014CC0`,
-- `sub_80155CC`): 21 ticks, then the change is done and the navi idles;
-- the Crosses note the side crossed.
local function settle(me: Object, s: State, seq: Sequence)
    if me.step_init == 0 then
        s.timer = 0x14
        me.step_init = 4
    end
    if not timer_ran_out(s) then
        return
    end
    if seq == "cross" or seq == "cross_beast" or seq == "beast_cross" then
        -- `sub_800AB2E`
        local record = system.state() :: Record
        record.crossed = true
    end
    finish(me)
end

-- One paused tick of the change: the action's update. (What it notes of
-- the round is the forms system's state, system.luau.)
function change.update(me: Object, s: State)
    local target = me:form_change_target()
    if target == nil then
        -- (A change to the base form is none.)
        me:set_navi_state("form_change", false)
        return
    end
    local current = battle.navi(me.alliance).form
    local seq = sequence(target.kind, current.kind)
    local step = me.step
    if step == PREPARE then
        prepare(me, s, seq, current)
    elseif step == VANISH then
        if seq == "cross" or seq == "beast_cross" then
            merging(me, s, seq, target)
        else
            vanish(me, s, seq, target)
        end
    elseif step == EMERGE then
        emerge(me, s, seq, target)
    elseif step == SETTLE then
        settle(me, s, seq)
    else
        error(string.format("form change step %#x runs off its table (sub_8014A38)", step))
    end
    if not me:navi_state("form_change_sprite_held") then
        me:step_sprite()
    end
end

return change
