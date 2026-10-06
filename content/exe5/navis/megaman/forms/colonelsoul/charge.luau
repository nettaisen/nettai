--!strict
-- ColonelSoul's charged shot (`colonelsoul/charge`): the original's weapon routine 0x14 (0x0800F7E4) sets 40 damage
-- and 10 more per point of buster Attack plus one (0x0800FE5E), its own counter byte and the slash's row 0, finds
-- its target (0x080BA54C: the first enemy body ahead in MegaMan's row; else three panels ahead, or none off the
-- field), and starts action 0x43 (0x080EECAA): MegaMan draws his sword (the counter window open, a sound) and on the
-- 13th tick divides the screen on the target: a slash over it and the two panels diagonally behind it (no spark, a
-- slash against navis) and its effect there. 21 ticks, then 5 of recovery, the sword put away.

local attachment = require("../../../../objects/attachment")
local collision = require("../../../../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../../../../lib/swords")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x16 and +0x17: the target's panel.
    target_x: number,
    target_y: number,
}

local HIT_PARAM = 0x94
local ANIM = 5
local SOUND = asset.sound("spin")
local DRAW_TICKS, SLASH_AT, RECOVER_TICKS = 21, 9, 5
local HIT_MOD = 3
-- An enemy body, by side (0x080BA5B0).
local ENEMY_BODY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
-- The slash: AIAttackVars +3's row of 0x080EED64 and its effect (0x48 on: sprite 10-1F), row 0, the routine's (the
-- only one that starts the action): the target and the two panels diagonally behind it.
local REGION = regions.vee
local SLASH = new.effect { sprite = asset.sprite("screen-divide"), anim = 0, palette = 0 }
-- The sword: attachment row 3 (the swords' blade), row 0x32 (the same in palette 6) while Chaos Unison's charge is
-- armed (0x080EC082).
local CHAOS_BLADE = attachment.look { sprite = asset.sprite("sword"), palette = 6 }

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

-- 0x080EECC4: the sword drawn; the slash on the count's 9; 21 ticks.
local function draw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(ANIM)
        local blade = if me.chaos_armed then CHAOS_BLADE else swords.blade
        attachment.attach(me, blade, "related", { anim = swords.blade_anim(me) })
        me:open_counter_window()
        battle.play_sound(SOUND)
        s.timer = DRAW_TICKS
        me.step_init = 4
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
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EED74: 5 ticks; then the sword put away and back to idle.
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
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "colonelsoul/charge/action",
    state = { timer = "u16", target_x = "u8", target_y = "u8" },
    -- 0x080EECAA
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            draw(me, s)
        elseif step == 4 then
            recover(me, s)
        else
            error(string.format("ColonelSoul's charged shot's step %#x reads past its table (0x080EECBC)", step))
        end
    end,
}

-- 0x0800F7E4
local function setup(navi: Object): Action
    navi.attack_damage = 40 + 10 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    local s = navi:action_state(charge.action) :: State
    s.target_x, s.target_y = target(navi)
    return charge.action
end

charge.weapon = new.weapon {
    -- The charge table's row 0x14 (0x0801CB34); Charge 5 reads on into row 0x15's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 90 },
    setup = setup,
}

return charge
