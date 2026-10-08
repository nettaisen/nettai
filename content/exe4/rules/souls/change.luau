--!strict
-- EXE4's soul change (MegaMan's action 0x0D, 0x080EBA44; its steps by 0x080EBA5C): with a soul's chip next in his hand,
-- the intake asks for it (./init, 0x0800B658) and the status routine starts it as the fight runs (0x08013B24: the
-- engine's unpaused form change), the first unpaused tick after the turn's banner. It dims the battle itself, rather
-- than run while it is paused as EXE5's and EXE6's do. Its target is the soul its side gave at OK
-- (`navi:form_change_target()`: rules/souls/custom). Each soul's chip makes its own (`change.make`: its chip's use,
-- the soul's form's `change`), with what the soul's does besides; the original's one action is compat's.
--
-- Its steps (AIAttackVars +0, the attack's step):
--
-- - 0 (0x080EBA70): the battle dimmed (battle flag 4) and MegaMan white, each tick; first: held (the frozen flag,
--   0x10000, which EXE4 keeps on a navi changing form), the dark soul's flag cleared (AIData +0x44's 0x1000: no dark
--   soul in a netbattle's code ported), the screen fading out (0x44 at 16), onto his future panel, the flash (effect
--   row 3, 16 pixels up) and its sound (0x129), no longer moving (flag 0x40) nor asked to revert (+0x44's 0x40), the
--   soul's own part (its change's `prepare`: WindSoul's north wind, 0x080EBAD2 for AIData +0x10 3), the emotion
--   window hidden (HUD task 20), his collision region gone; then once the fade is done the HP box and the gauge hidden
--   too (tasks 0 and 1).
-- - 4 (0x080EBB14): the soul's image (./image) and 60 ticks; at 10 the second flash and its sound (0x158), and the
--   soul's screen flash (JunkSoul's, 0x080EBB50: 0x080E2B38).
-- - 8 (0x080EBB6C): white; first: MegaMan in the soul (NaviStats +0x24: his sprite in it, 0x0800B90A, his form; he
--   is taken for his navi still), its overlay (0x0800B7E8: the soul's own, its form's `put_on`), the dark soul's ticks (0x0800B93C:
--   AIData +0x2C, read only in the dark soul), his statuses ended (0x08013218), 10 ticks, and the cameras' shake
--   (0x080EBC00 by soul: GutsSoul's and FireSoul's, 2 for 50 ticks).
-- - 0xC (0x080EBC20): the screen fading in (0x40 at 16); once done the HUD back (tasks 0, 1 and 20; in a link battle's
--   last turns 0 and 20, no gauge), the status reset (0x0800E05C: the soul's turns, ./init, and its own routine,
--   0x0800E0A0 by soul) and the charge's state (0x0800C5EA).
-- - 0x10 (0x080EBC68): 20 ticks; then no longer held, the hand's next chip (sub_800FC7C: past the soul's), the battle
--   no longer dimmed, his collision region back, no soul asked for (AIData +0x10 0xFF), and the attack's end
--   (0x0800C9FC).

local hooks = require("./hooks")
local image = require("./image")
local regions = require("@exelib/regions")
local turns = require("./turns")

local change = {}

export type State = {
    -- AIAttackVars+0x10: ticks left in the current step.
    timer: number,
}

local CHANGE_SOUND = asset.sound("soul-change")
local UNITE_SOUND = asset.sound("soul-unite")
-- Effect row 3, the deletion's flash.
local FLASH = new.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 }
-- The fades' speed.
local FADE_SPEED = 16
-- The cameras' shake (0x080EBC00's rows: magnitude 2, 50 ticks).
local SHAKE_MAGNITUDE, SHAKE_TICKS = 2, 50
-- The lockout its end hands on (the status routine's +0x75 as it starts the change, 0x08013B1C; the attack's end puts
-- it in the lockout, 0x0800CA0C): his chips wait a tick past the next.
local END_LOCKOUT = 2

-- The steps, in the attack's step.
local PREPARE, IMAGE, EMERGE, FADE_IN, SETTLE = 0, 4, 8, 0xC, 0x10

-- What a soul's change does besides the steps every soul's takes.
export type Spec = {
    -- Its soul's chip's key: the change is the action `<id>/change`, the soul's image the kind `<id>/image`.
    id: string,
    -- The cameras shake as MegaMan emerges in the soul (0x080EBC00's table: GutsSoul's and FireSoul's).
    shake: boolean?,
    -- The screen flashes with the image's second flash (0x080EBB50: JunkSoul's).
    flash: boolean?,
    -- What else the first step does, as it starts (WindSoul's north wind).
    prepare: ((navi: Object) -> ())?,
}

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The flash (effect row 3) 16 pixels up, which runs while the battle is dimmed (its pool's objects do).
local function flash(me: Object)
    battle.effect(me.pos + Vec3.new(0, 0, 0x10_0000), FLASH)
end

-- 0x080EBA70
local function prepare(me: Object, soul: FormDef, spec: Spec)
    battle.set_dimmed(true)
    me.sprite.white = true
    if me.step_init == 0 then
        me.attack_lockout = END_LOCKOUT
        me:set_status("frozen", true)
        battle.screen_fade("transform_out", FADE_SPEED)
        me:snap_to_future_panel()
        flash(me)
        battle.play_sound(CHANGE_SOUND)
        me:set_status("moving", false)
        me:set_request("revert_form", false)
        if spec.prepare then
            spec.prepare(me)
        end
        battle.show_hud({ "emotion_window" }, false)
        me.collision:set_region(nil)
        me.step_init = 4
    end
    if not battle.screen_fading() then
        battle.show_hud({ "hp_box", "gauge" }, false)
        set_step(me, IMAGE)
    end
end

-- 0x080EBB14
local function show_image(me: Object, s: State, soul: FormDef, spec: Spec, spawn_image: (Object, FormDef) -> Object?)
    if me.step_init == 0 then
        spawn_image(me, soul)
        s.timer = 60
        me.step_init = 4
    end
    if s.timer == 10 then
        battle.play_sound(UNITE_SOUND)
        flash(me)
        if spec.flash == true then
            battle.palette_flash(0, 0x28, true, false)
        end
    end
    s.timer -= 1
    if int.i16(s.timer) < 0 then
        set_step(me, EMERGE)
    end
end

-- 0x080EBB6C
local function emerge(me: Object, s: State, soul: FormDef, spec: Spec)
    me.sprite.white = true
    if me.step_init == 0 then
        local stats = battle.navi(me.alliance)
        hooks.finish(me, stats.form)
        me:load_form_sprite(soul)
        stats.form = soul
        hooks.identity(me, soul)
        hooks.start(me, soul)
        me.related1 = nil
        s.timer = 10
        me:end_statuses()
        me.sprite.white = true
        if spec.shake == true then
            battle.shake_camera(SHAKE_MAGNITUDE, SHAKE_TICKS)
        end
        me.step_init = 4
    end
    s.timer -= 1
    if int.i16(s.timer) > 0 then
        return
    end
    set_step(me, FADE_IN)
end

-- 0x080EBC20
local function fade_in(me: Object)
    if me.step_init == 0 then
        battle.screen_fade("transform_in", FADE_SPEED)
        me.step_init = 4
        return
    end
    if battle.screen_fading() then
        return
    end
    battle.show_hud(if battle.late_turns() then { "hp_box", "emotion_window" } else { "hp_box", "gauge", "emotion_window" }, true)
    turns.start(battle.navi(me.alliance).form)
    hooks.settle(me)
    set_step(me, SETTLE)
end

-- 0x080EBC68
local function settle(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 20
        me.step_init = 4
    end
    s.timer -= 1
    if int.i16(s.timer) >= 0 then
        return
    end
    me:set_status("frozen", false)
    battle.advance_hand(me.alliance)
    battle.set_dimmed(false)
    me.collision:set_region(regions.single)
    me:set_navi_state("form_change", false)
    me:exit_attack()
end

-- A soul's change, its chip's use (the soul's form's `change`): MegaMan's action 0x0D, as `spec` says.
function change.make(spec: Spec): Action
    local spawn_image = image.make(spec.id .. "/image")
    return new.action {
        id = spec.id .. "/change",
        state = { timer = "u16" },
        -- One tick of the change.
        update = function(me: Object, s: State)
            local soul = me:form_change_target()
            if soul == nil or soul.soul == nil then
                error("EXE4's soul change asked for no soul (0x080EBA44: AIData +0x10)")
            end
            local step = me.step
            if step == PREPARE then
                prepare(me, soul, spec)
            elseif step == IMAGE then
                show_image(me, s, soul, spec, spawn_image)
            elseif step == EMERGE then
                emerge(me, s, soul, spec)
            elseif step == FADE_IN then
                fade_in(me)
            elseif step == SETTLE then
                settle(me, s)
            else
                error(string.format("the soul change's step %#x runs off its table (0x080EBA5C)", step))
            end
        end,
    }
end

return change
