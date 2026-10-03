--!strict
-- BN5's Soul Unison change (0x08011F74, BN6's `sub_8014A38`'s place): while the battle is paused at a turn's start,
-- MegaMan unites with the soul his side asked for (the turn's transform record: `navi:form_change_target()`, its
-- turns and Chaos Unison `navi:form_change_soul()`). The turn-start sequencer (the framework's) waits for it. Every
-- soul names this action as its `change` (./system).
--
-- Its steps (0x08011F94, on the attack's step):
--
-- - 0 (0x08011FAC): onto his future panel, facing the default way, the charge dropped, his moves stopped; a flash
--   (effect 3) and a sound; his sprite held.
-- - 4 (0x08012010): the soul's image comes in (objects/soul-image); 50 ticks later another flash and the soul's
--   sound (Chaos Unison's its own); 61 ticks.
-- - 8 (0x08012090): MegaMan in the soul (his sprite by the soul, the soul's turns, his form); 10 ticks later the
--   status reset, his anger ends, his mood back to 0x80 (unless it was 0), the soul's charge state cleared and his
--   invulnerability ended.
-- - 0x10 (0x08012196): 21 ticks, then the change is done and he idles.

local effects = require("@bn6/lib/effects")
local image = require("../../objects/soul-image/image")

local change = {}

export type State = {
    -- AIAttackVars+0x10: ticks left in the current step.
    timer: number,
}

-- The souls system's state (./system).
type Souls = { turns: number, chaos: boolean }

local FLASH_SOUND = asset.sound("bn5:log-in")
local UNISON_SOUND = asset.sound("bn5:cross-change")
local CHAOS_SOUND = asset.sound("bn5:bug")

-- The steps, in the attack's step.
local PREPARE, IMAGE, EMERGE, CHAOS, SETTLE = 0, 4, 8, 0xC, 0x10

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The flash (effect 3, 16 pixels up), which runs while the battle is paused.
local function flash(me: Object)
    local e = battle.effect(me.pos + Vec3.new(0, 0, 0x10_0000), effects.deletion)
    if e then
        e.run_while_paused = true
    end
end

-- 0x08011FAC
local function prepare(me: Object)
    me.sprite.white = true
    me:snap_to_future_panel()
    me:face_default()
    me:reset_charge()
    flash(me)
    battle.play_sound(FLASH_SOUND)
    me:stop_moving()
    me:set_navi_state("form_change_sprite_held", true)
    set_step(me, IMAGE)
end

-- 0x08012010
local function show_image(me: Object, s: State, soul: number, chaos: boolean)
    if me.step_init == 0 then
        image.spawn(me, soul)
        if chaos then
            image.chaos_aura(me)
        end
        s.timer = 60
        me.step_init = 4
    end
    if s.timer == 10 then
        battle.play_sound(if chaos then CHAOS_SOUND else UNISON_SOUND)
        flash(me)
    end
    s.timer -= 1
    if int.i16(s.timer) < 0 then
        set_step(me, EMERGE)
    end
end

-- 0x08012090
local function emerge(me: Object, s: State, target: FormDef, chaos: boolean)
    me.sprite.white = true
    if me.step_init == 0 then
        me:set_navi_state("form_change_sprite_held", false)
        me.related1 = nil
        me.overlay = nil
        me:load_form_sprite(target)
        s.timer = 10
        me:end_statuses()
        local turns, is_chaos = me:form_change_soul()
        local souls = system.state() :: Souls
        souls.turns, souls.chaos = turns, is_chaos
        battle.navi(me.alliance).form = target
        me.step_init = 4
    end
    s.timer -= 1
    if int.i16(s.timer) > 0 then
        return
    end
    me:reset_status()
    -- 0x08011A94: the mood back to 0x80 (BN5's setter leaves a mood of 0), the anger over.
    local stats = battle.navi(me.alliance)
    local mood = stats.mood
    me:end_anger()
    if mood == 0 then
        stats.mood = 0
    end
    -- 0x0802C126: the soul's charge state cleared.
    me:set_navi_state("no_charge", false)
    me:clear_invulnerable()
    set_step(me, if chaos then CHAOS else SETTLE)
end

-- 0x0801216C: Chaos Unison's 11 ticks.
local function chaos_step(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 0
        me.step_init = 4
    end
    s.timer += 1
    if s.timer >= 10 then
        set_step(me, SETTLE)
    end
end

-- 0x08012196
local function settle(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 20
        me.step_init = 4
    end
    s.timer -= 1
    if int.i16(s.timer) >= 0 then
        return
    end
    me:set_navi_state("form_change", false)
    me:exit_attack()
end

-- One paused tick of the change: the action's update.
function change.update(me: Object, s: State)
    local target = me:form_change_target()
    if target == nil or target.soul == nil then
        error("BN5's soul change asked for no soul (0x08011F74)")
    end
    local soul = (target.soul :: any).number :: number
    local _, chaos = me:form_change_soul()
    local step = me.step
    if step == PREPARE then
        prepare(me)
    elseif step == IMAGE then
        show_image(me, s, soul, chaos)
    elseif step == EMERGE then
        emerge(me, s, target, chaos)
    elseif step == CHAOS then
        chaos_step(me, s)
    elseif step == SETTLE then
        settle(me, s)
    else
        error(string.format("the soul change's step %#x runs off its table (0x08011F94)", step))
    end
    if not me:navi_state("form_change_sprite_held") then
        me:step_sprite()
    end
end

return change
