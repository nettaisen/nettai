--!strict
-- BN5's dark MegaMan's last stand: BN5's action 0x30 (0x080EFE22), the roles' `volley`. A player MegaMan (AI index 0)
-- of BN5's emotion 5 (a mood of 0, a dark MegaMan's) whom a loss of HP brings to 0 holds at 1 HP, once a battle,
-- and asks for the request 0x40000000 (0x0802C16C, from object_subtractHP, 0x0800C6E0, and applyDamageToPlayer,
-- 0x080185A2: the status section's `hp_loss`). This is what the request starts. Its steps (0x080EFE34, on the
-- attack's step):
--
-- - 0 (0x080EFE48): the battle dims and the screen fades out; he shows white, no hit reaches him (untouchable), he
--   stands on his future panel, a flash (effect 3) and a sound, his moves, his reactions and his attachments
--   dropped; his side marked (its statistic 1: the stand is spent). Once the fade is done the HUD's gauge, HP box
--   and emotion window go.
-- - 4 (0x080EFF10): his dark self leaves him and spirals back into him (actor object 0x2E: ../souls/shade's); 50
--   ticks later another flash and a sound; 61 ticks.
-- - 8 (0x080EFF58): the computer drives him for 720 ticks (rules/computer-navi, as DarkInvs's drive drives its user:
--   chips/darkinvs; the drive's end is BN5's action 0x49, the roles' `stun_strike`), his statuses and his anger
--   ended.
-- - 0xC (0x080EFF78): the HUD back (in the last turns without the gauge) and the screen fades in; then he is no
--   longer in the stand, the battle undims and he idles.
--
-- His sprite steps every tick, paused or not (`sub_801BCD0`, 0x080187D4).

local shade = require("../souls/shade")

local survival = {}

export type State = {
    -- AIAttackVars+0x10: ticks left of the step.
    timer: number,
}

-- The flash: BN5's effect 3 (0x080DFC94's row 3: 14-03).
local FLASH = define.effect { sprite = asset.sprite("bn5:deletion"), anim = 0, palette = 0 }
local FLASH_SOUND = asset.sound("bn5:log-in")
local CHANGE_SOUND = asset.sound("bn5:cross-change")
-- The side's statistic the stand marks (0x0800931C's byte 1, which 0x0802C16C reads).
local STOOD = 1
-- How long the computer drives him (0x080EFF64).
local DRIVE = 0x2D0
-- The screen fades' speed.
local FADE_SPEED = 0x10
-- The steps, on the attack's step.
local FADE_OUT, SHADE, TAKE_OVER, FADE_IN = 0x0, 0x4, 0x8, 0xC
-- The shade's step: its ticks (to -1), and the ticks left at the second flash.
local SHADE_TICKS, SECOND_FLASH = 60, 10
-- The HUD parts the stand hides and shows (0x0801A418 and 0x0801A398 with 0x4090: draw tasks 4, 7 and 14); in the
-- last turns (0x0800916A) it shows them without the gauge (0x4080).
local HUD: { HudPart } = { "gauge", "hp_box", "emotion_window" }
local HUD_LAST_TURNS: { HudPart } = { "hp_box", "emotion_window" }

-- His dark self (actor object 0x2E, 0x080C0EA0; spawned by 0x080C0FE2): Chaos Unison's shade's code, but a pixel up
-- and on rather than two, its ground shadow kept, and wearing what MegaMan's record puts on while it is out.
local dark_self = shade.make {
    id = "bn5:dark-survival/shade",
    name = "the dark survival's shade",
    phases = 0x080C0F14,
    lift = 0x1_0000,
    shadow = "ground",
    wears = true,
}

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The flash (effect 3, 16 pixels up), which runs while the battle is paused.
local function flash(me: Object)
    local e = battle.effect(me.pos + Vec3.new(0, 0, 0x10_0000), FLASH)
    if e then
        e.run_while_paused = true
    end
end

-- 0x0800916A: the netbattle's last turns (from its 15th custom screen; never in the battle flag 0x40 mode).
local function last_turns(): boolean
    return not battle.per_player_gauges() and battle.late_turns()
end

-- 0x080EFE48
local function fade_out(me: Object)
    if me.step_init == 0 then
        battle.set_dimmed(true)
        me:set_request("volley", false)
        me:set_navi_state("volley", true)
        me.sprite.white = true
        me:set_status("untouchable", true)
        battle.screen_fade("transform_out", FADE_SPEED)
        me:snap_to_future_panel()
        flash(me)
        battle.play_sound(FLASH_SOUND)
        -- Not moving, flinching, paralyzed, sliding or dragged, nor asked to slide or be dragged (flags 0x101C40,
        -- 0x110; +0x1F).
        me:stop_moving()
        me:set_status("drag", false)
        me:set_request("drag", false)
        me:set_animation(0)
        me.related1 = nil
        me.overlay = nil
        -- 0x080C374E: what he wears reloads its animation, and steps through the pause (its Param3).
        local part = me.related2
        if part then
            part.anim_loaded = 0xFF
        end
        me:overlay_stepping("keep")
        battle.set_side_stat(me.alliance, STOOD, 1)
        me.step_init = 4
    end
    if not battle.screen_fading() then
        battle.show_hud(HUD, false)
        me:overlay_stepping("normal")
        set_step(me, SHADE)
    end
end

-- 0x080EFF10
local function dark_shade(me: Object, s: State)
    if me.step_init == 0 then
        dark_self.spawn(me)
        s.timer = SHADE_TICKS
        me.step_init = 4
    end
    if s.timer == SECOND_FLASH then
        battle.play_sound(CHANGE_SOUND)
        flash(me)
    end
    local left = s.timer - 1
    s.timer = int.u16(left)
    if left < 0 then
        set_step(me, TAKE_OVER)
    end
end

-- 0x080EFF58: white; the drive (0x0802C03A and 0x0802C110: the computer-navi AI's state from the start, which the
-- system does on finding the drive new; the ticks, the AI on, the no-charge state); his statuses and his anger
-- ended (0x08017068, 0x08011A94).
local function take_over(me: Object)
    me.sprite.white = true
    me.no_charge_timer = DRIVE
    me.computer_driven = true
    me:set_navi_state("no_charge", true)
    me:end_statuses()
    me:end_anger()
    set_step(me, FADE_IN)
end

-- 0x080EFF78: (battle flag 0x20 is cleared too, which nothing reads.)
local function fade_in(me: Object)
    if me.step_init == 0 then
        battle.show_hud(if last_turns() then HUD_LAST_TURNS else HUD, true)
        battle.screen_fade("transform_in", FADE_SPEED)
        me.step_init = 4
    end
    if not battle.screen_fading() then
        me:set_navi_state("volley", false)
        battle.set_dimmed(false)
        me:exit_attack()
    end
end

-- 0x080EFE22
local function update(me: Object, s: State)
    local step = me.step
    if step == FADE_OUT then
        fade_out(me)
    elseif step == SHADE then
        dark_shade(me, s)
    elseif step == TAKE_OVER then
        take_over(me)
    elseif step == FADE_IN then
        fade_in(me)
    else
        error(string.format("the dark survival's step %#x reads past its table (0x080EFE34)", step))
    end
    me:step_sprite()
end

survival.action = define.action { id = "bn5:dark-survival", state = { timer = "u16" }, update = update }

return survival
