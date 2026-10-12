--!strict
-- MetalSoul's charged shot (`metalsoul/charged-shot`): weapon routine 0x13 (0x0800CE28: 150 damage, counter byte 0xA8,
-- the attack parameters 0x06050601, every other attack variable 0) and the fist it starts (action 0x14, 0x080EC0A4,
-- its alone): the navi swings (animation 10) with a counter window open, sounding as it starts (0xB7, MetalMan's
-- wind-up); on its eleventh tick a hit on the panel ahead (the parameters: region 1, the anchor alone; spark 6, the
-- heavy hit's; at navis; breaking), and where that panel is solid a burst on it (effect row 6), a shake (0, 30 ticks)
-- and the punch's sound (0xA2); 18 ticks swinging, then 31 recovering, it exits the attack, handing the lockout on
-- (0x0800C9FC).

local collision = require("../../../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../../../lib/sparks")

local charge = {}

local SWING, RECOVER = 0x0, 0x4
local ANIM = 10
-- Its ticks: the swing counts up from 0, striking at 10, to 18; the recovery down from 30 past 0 (`bge`).
local STRIKE_AT, SWING_END, RECOVER_TICKS = 10, 17, 30
-- The hit's height and hit modifier (0x080CD7E2's r3 and r7).
local STRIKE_Z = 16 * 0x10000
local HIT_MOD = 0x23
-- What a solid panel shows: effect row 6 (0x080E1698, through 0x080E18EE: 14-0a, at the panel's center, height 0).
local BURST = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 }
local SHAKE, SHAKE_TICKS = 0, 30
local SOUND_SWING = asset.sound("metal-fist")
local SOUND_PUNCH = asset.sound("metal-punch")

type State = { timer: number }

-- 0x080EC0C4.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM
        me:open_counter_window()
        me:set_status("using_action", true)
        battle.play_sound(SOUND_SWING)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == STRIKE_AT then
        local x, y = me.panel_x + me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.attack_element,
            z = STRIKE_Z,
            region = regions.single,
            hit_effect = sparks.heavy,
            target = collision.hits_navis,
            self_type = collision.breaking,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = HIT_MOD,
        })
        if field.solid(x, y) then
            local cx, cy = field.center(x, y)
            battle.effect(Vec3.new(cx, cy, 0), BURST)
            battle.shake_camera(SHAKE, SHAKE_TICKS)
            battle.play_sound(SOUND_PUNCH)
        end
    end
    s.timer += 1
    if s.timer > SWING_END then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080EC146.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "metalsoul/charged-shot/action",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == SWING then
            swing(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("MetalSoul's fist's phase %#x reads past its phase table", step))
        end
    end,
}

-- 0x0800CE28.
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0xA8
    navi.attack_damage = 150
    return charge.action
end

charge.weapon = new.weapon {
    id = "metalsoul/charged-shot",
    -- The charge table's row 0x13 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x14's
    -- first entry.
    charge_ticks = { 80, 80, 80, 80, 80, 120 },
    setup = setup,
}

return charge
