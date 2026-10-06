--!strict
-- Colonel's charged attack, the Screen Divide (`colonel/charge`): the original's weapon routine 0x46 (0x0800FC0C) takes
-- the damage his level gives (the damage rows' row 13, lib/navi_level) and his counter byte, no wait and no warning,
-- finds his target (0x080BA54C: the first enemy body ahead in his row; else three panels ahead, or none off the field)
-- and starts his own action 7 (0x080F89AE; ColonelSoul's charged shot, action 0x43, is MegaMan's cut of it). He stands
-- ready (animation 24) for the wait the attack gives, then draws his sword (the counter window open) and on its 10th
-- tick divides the screen on the target: a slash over it and the two panels diagonally nearer him (a slash against
-- navis, no spark) and its effect there. 21 ticks, then 5 of recovery, the sword put away.
--
-- The action is also his in an auto battle: with a wait, and warning of the panels it will cut (they flash four
-- ticks in eight) unless the attack says not to. The player's routine gives no wait and no warning. (Its start also
-- calls 0x080E9FD2, the operation battle's mark on the target, which isn't ported: lib/swords.)

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local navi_level = require("../../lib/navi_level")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local swords = require("../../lib/swords")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x12: ticks since the action's start (the warning's flash).
    count: number,
    -- AIAttackVars +0x16 and +0x17: the target's panel.
    target_x: number,
    target_y: number,
    -- AIAttackVars +0x18: the wait before the sword is drawn.
    wait: number,
    -- AIAttackVars +0x30: no warning.
    unwarned: boolean,
}

local HIT_PARAM = 0x94
-- The damage rows' row 13, by his level.
local DAMAGE: { number } = { 50, 50, 60, 60, 70, 70, 70 }
local ANIM_READY, ANIM_DRAW = 24, 5
local SOUND = asset.sound("spin")
local DRAW_TICKS, SOUND_AT, SLASH_AT, RECOVER_TICKS = 21, 17, 12, 5
local COUNTER_TICKS = 16
local HIT_MOD = 3
-- An enemy body, by side (0x080BA5B0).
local ENEMY_BODY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
-- The slash: AIAttackVars +3's row of 0x080F8AF4 (0x080F8A30 and 0x080F8B00 for the warning) and its effect (0x48
-- on), row 0, the routine's: the target and the two panels diagonally behind it.
local REGION = regions.vee
local SLASH = new.effect { sprite = asset.sprite("screen-divide"), anim = 0, palette = 0 }
-- His sword's animation (the attachment's spawn word: row 3, animation 7).
local BLADE_ANIM = 7

-- The steps, on the attack's step.
local READY, DRAW, RECOVER = 0x0, 0x4, 0x8

-- 0x080BA54C
local function target(me: Object): (number, number)
    local y = me.panel_y
    local x = me.panel_x + me.facing
    while field.valid(x, y) do
        if bit32.btest(field.flags(x, y), ENEMY_BODY[me.alliance]) then
            return x, y
        end
        x += me.facing
    end
    x = me.panel_x + me.facing * 3
    if not field.valid(x, y) then
        x = 0
    end
    return x, y
end

-- The warning: unless the attack says not to, the slash's panels flash while bit 2 of the count is clear (by the
-- side's direction: the routine passes the side and flip halfword unmasked, and nothing turns him here).
local function warn(me: Object, s: State)
    if not s.unwarned and not bit32.btest(s.count, 4) then
        panels.highlight_region(s.target_x, s.target_y, REGION, panels.side_direction(me.alliance))
    end
end

-- 0x080F89D4: he stands ready for the wait.
local function ready(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM_READY)
        s.timer = s.wait
        s.count = 0
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.step, me.step_init = DRAW, 0
    end
    warn(me, s)
end

-- 0x080F8A34: the sword drawn; its sound on the count's 17, the slash on its 12; 21 ticks.
local function draw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(ANIM_DRAW)
        attachment.attach(me, swords.blade, "overlay", { anim = BLADE_ANIM })
        me:open_counter_window(COUNTER_TICKS)
        s.timer = DRAW_TICKS
        me.step_init = 4
    end
    if s.timer == SOUND_AT then
        battle.play_sound(SOUND)
    end
    if s.timer == SLASH_AT then
        battle.hitbox(me, {
            panel_x = s.target_x,
            panel_y = s.target_y,
            element = 0,
            z = 0,
            region = REGION,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = HIT_MOD,
        })
        local cx, cy = field.center(s.target_x, s.target_y)
        battle.effect(Vec3.new(cx, cy, 0), SLASH, bit32.bxor(me.alliance, me.flip))
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me:set_status("using_action", false)
        me.step, me.step_init = RECOVER, 0
    end
    warn(me, s)
end

-- 0x080F8B04: 5 ticks; then the sword put away and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.related1 = nil
        me.overlay = nil
        me:set_animation(0)
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "colonel/charge/action",
    state = { timer = "u16", count = "u16", target_x = "u8", target_y = "u8", wait = "u16", unwarned = "bool" },
    -- 0x080F89AE: the step, then the count.
    update = function(me: Object, s: State)
        local step = me.step
        if step == READY then
            ready(me, s)
        elseif step == DRAW then
            draw(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("Colonel's Screen Divide's step %#x reads past its table (0x080F89C8)", step))
        end
        s.count = bit32.band(s.count + 1, 0xFFFF)
    end,
}

-- 0x0800FC0C (the attack's charged byte, lockout and bonus are left as the last attack had them)
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    local s = navi:action_state(charge.action) :: State
    s.wait = 0
    s.target_x, s.target_y = target(navi)
    s.unwarned = true
    return charge.action
end

charge.weapon = new.weapon {
    id = "colonel/charge",
    -- The charge table's row 0x46 (0x0801CD28), by Charge 0 to 5: past 4 it reads on into row 0x47's first entry.
    charge_ticks = { 120, 115, 110, 105, 100, 160 },
    setup = setup,
}

return charge
