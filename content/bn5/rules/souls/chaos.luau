--!strict
-- BN5's Chaos Unison failure (action 0x39, 0x080EE63C; the role `actions.chaos_failure`, which the engine starts
-- when an armed charge is released out of the chaos cycle's window: the navi uninterruptible, `set_attack` slot 6).
-- The battle dims (battle flag 4 alone, once no dimming runs), MegaMan flashes white every other pair of ticks
-- while he crouches (animation 12, 6 ticks) and fades (animation 1 and the fade sound, 25 ticks), and on the fade's
-- seventeenth tick the Dark MegaMen across from him leave (rules/computer-navi) and a new one comes onto a solid,
-- empty panel of the other side's area (navis/dark-megaman, driven by BN5's computer navis, his summoner's side's
-- system). Then MegaMan is back in his base form, as a soul's revert leaves him (./revert, but in place, with no
-- flash or sound), his attack requests and his uninterruptibility are dropped, the battle undims and he idles.

local base = require("../../navis/megaman/forms/base")
local computer_navi = require("../computer-navi/system")
local dark_megaman = require("../../navis/dark-megaman/identity")
local hooks = require("./hooks")
local panels = require("../computer-navi/panels")

local chaos = {}

export type State = {
    -- AIAttackVars+0x10: ticks left of the step.
    timer: number,
}

local FADE = asset.sound("bn5:fade")

-- The panels Dark MegaMan may come onto, by the user's side (`sub_80129F4`'s table, 0x0801025C): the other side's
-- area, solid and with no body on it.
local AREA: { { number } } = { { 0x30, 0x0F880080 }, { 0x10, 0x0F8800A0 } }

-- The steps, on the attack's step.
local CROUCH, VANISH, REVERT = 0x0, 0x4, 0x8

-- The ticks so far (AIAttackVars+0x12, counted on from what it held): every other pair, MegaMan shows white.
local function flash(me: Object)
    me.attack_count = bit32.band(me.attack_count + 1, 0xFFFF)
    if bit32.band(me.attack_count, 2) ~= 0 then
        me.sprite.white = true
    end
end

-- 0x080EE65C: once no dimming runs, the battle dims and MegaMan crouches for 6 ticks.
local function crouch(me: Object, s: State)
    if me.step_init == 0 then
        if battle.dimmed() then
            return
        end
        battle.set_dimmed(true)
        me:set_animation(12)
        s.timer = 5
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.step = VANISH
        me.step_init = 0
    end
    flash(me)
end

-- `sub_80129F4`: a panel of the other side's area at random (nil: none).
local function landing(me: Object): (number?, number?)
    local area = AREA[bit32.band(me.alliance, 1) + 1]
    return panels.pick(panels.filtered(area[1], area[2]))
end

-- 0x080EE69A: MegaMan fades for 25 ticks; on the seventeenth, Dark MegaMan comes.
local function vanish(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(1)
        battle.play_sound(FADE)
        s.timer = 24
        me.step_init = 4
    end
    if s.timer == 8 then
        computer_navi.dismiss(me.alliance)
        local x, y = landing(me)
        if x ~= nil then
            -- (BN5's 0x08006AAE: NameID 0x18D for the other side; the navi type's tick runs for him from the start.)
            local navi = battle.spawn_navi(dark_megaman, x, y :: number, bit32.bxor(me.alliance, 1), me, computer_navi.system)
            if navi then
                navi.ticked = true
            end
        end
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.step = REVERT
        me.step_init = 0
    end
    flash(me)
end

-- 0x080EE6FC: back in the base form (the soul's end hook, the base form's sprite, the status reset, the base form's
-- start hook), his attachments gone; the requests 0x3D and his uninterruptibility dropped; the battle undims.
local function revert(me: Object, _s: State)
    local stats = battle.navi(me.alliance)
    hooks.finish(me, stats.form)
    me:load_form_sprite(base)
    stats.form = base
    hooks.settle(me)
    hooks.start(me, base)
    me.related1 = nil
    me.overlay = nil
    for _, request in { "buster", "chip", "charged_chip", "back_special", "forced_charged_shot" } do
        me:set_request(request :: Request, false)
    end
    me:set_navi_state("uninterruptible", false)
    battle.set_dimmed(false)
    me:exit_attack()
end

-- 0x080EE63C
local function update(me: Object, s: State)
    local step = me.step
    if step == CROUCH then
        crouch(me, s)
    elseif step == VANISH then
        vanish(me, s)
    elseif step == REVERT then
        revert(me, s)
    else
        error(string.format("the chaos failure's step %#x reads past its table (0x080EE650)", step))
    end
end

chaos.failure = define.action { id = "bn5:souls/chaos-failure", state = { timer = "u16" }, update = update }

return chaos
