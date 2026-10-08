--!strict
-- MegaMan's weapons the NaviCust's weapon bug (bug 8, rules/navicust: NaviStats +0x0A = 0x1F, 0x20, 0x21 by its level,
-- 0x08042E44) sets his charged shot to, the taunt (./taunt) its third: weapon routines 0x1F and 0x20.
--
-- - The rock cube (`megaman/rock-cube`, routine 0x1F, 0x0800CE7C: 200 damage, hit parameter 0xB2, its rock row 1;
--   action 0x73, 0x080EEA56): the navi stands (animation 0x12, its load not forced; a counter window) 5 ticks, then
--   with Rush's sound (0xAA) a rock cube rises on the panel ahead (../../../objects/rock, row 1: 200 HP, the cube's
--   standing animation 1, the panel crack's sound as it breaks), one of the side's field objects; 31 ticks, and back to
--   idle.
-- - The bubble (`megaman/bubble`, routine 0x20, 0x0800CFD0: 100 aqua damage; action 0x29, 0x080ED02A; patch card 108's
--   B button too): the navi (animation 0x12) on its second tick, with Tango's arrival sound (0x15A), hits the panel
--   ahead 16 pixels up (the region word 0x04050201: the panel, the aqua spark, an attack hitting navis; hit modifier 1)
--   and holds a bubble (attachment row 0x16: 0c-20 in palette 0, lifted a pixel); after 5 ticks the recovery, 11 ticks,
--   which a move may cut short, as the taunt's.

local attachment = require("../../../objects/attachment")
local collision = require("../../../rules/collision")
local regions = require("@exelib/regions")
local rock = require("../../../objects/rock")
local sparks = require("../../../lib/sparks")

local bug_shots = {}

-- The charge table's rows 0x1F and 0x20 (0x080181EE, 0x080181F8), by Charge 0 to 4; Charge 5 reads on into the next
-- row's first entry.
local ROCK_CUBE_CHARGE = { 300, 250, 200, 150, 100, 120 }
local BUBBLE_CHARGE = { 120, 120, 120, 120, 120, 180 }

-- The rock cube: row 1 of 0x080D89C0 (the cube, 200 HP).
local CUBE = rock.variant {
    anim = 1,
    hp = 200,
    element = "null",
    debris_palette = 0,
    break_sound = asset.sound("panel-crack"),
    identity = nil,
}
local PLACE_SOUND = asset.sound("place")
local CUBE_WAIT, CUBE_RECOVER = 4, 0x1E

-- The bubble (attachment row 0x16), its sound, its hit.
local BUBBLE = attachment.look { sprite = asset.sprite("bubble"), palette = 0, lift = 1 }
local BUBBLE_SOUND = asset.sound("arrive")
local BUBBLE_Z = 0x10_0000
local BUBBLE_HIT_MOD = 1

type State = { timer: number }

-- The recovery the taunt's and the bubble's share (0x080ED0B8, 0x080ED18C): its end exits the attack and a move may
-- still start, as may one before.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 10
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me.related1 = nil
        me:exit_attack()
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir == 0 or me:step_target(dir) == nil then
        return
    end
    me.related1 = nil
    me:exit_attack()
    me:start_move(dir)
end

bug_shots.rock_cube_action = new.action {
    id = "megaman/rock-cube/action",
    state = { timer = "i16" },
    -- 0x080EEA56.
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0x0 then
            -- 0x080EEA78: 5 ticks standing.
            if me.step_init == 0 then
                me.anim = 0x12
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = CUBE_WAIT
                me.step_init = 4
            end
            s.timer -= 1
            if s.timer < 0 then
                me.step, me.step_init = 0x4, 0
            end
        elseif step == 0x4 then
            -- 0x080EEAA6: the cube on the panel ahead (its parameter: row 1, slot class 0, rising).
            battle.play_sound(PLACE_SOUND)
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            rock.spawn(me.panel_x + me.facing, me.panel_y, me.alliance, { variant = CUBE, class = 0, entrance = "rise" }, damage)
            me.step, me.step_init = 0x8, 0
        elseif step == 0x8 then
            -- 0x080EEAC8: 31 ticks, then back to idle.
            if me.step_init == 0 then
                s.timer = CUBE_RECOVER
                me.step_init = 4
            end
            s.timer -= 1
            if s.timer < 0 then
                me:exit_attack()
            end
        else
            error(string.format("the rock cube's phase %#x reads past its table (0x080EEA6C)", step))
        end
    end,
}

bug_shots.bubble_action = new.action {
    id = "megaman/bubble/action",
    state = { timer = "i16" },
    -- 0x080ED02A.
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0x0 then
            -- 0x080ED048.
            if me.step_init == 0 then
                me:set_animation(0x12)
                me:set_status("using_action", true)
                me.step_init = 4
                s.timer = 0
            end
            if s.timer == 1 then
                battle.play_sound(BUBBLE_SOUND)
                local word = me.attack_damage + me.hit_param * 0x10000
                battle.hitbox(me, {
                    panel_x = me.panel_x + me.facing,
                    panel_y = me.panel_y,
                    element = me.attack_element,
                    z = BUBBLE_Z,
                    region = regions.single,
                    hit_effect = sparks.aqua,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = bit32.band(word, 0xFFFF),
                    stamina = bit32.rshift(word, 16),
                    hit_mod = BUBBLE_HIT_MOD,
                })
                attachment.attach(me, BUBBLE, "related", 0)
            end
            s.timer += 1
            if s.timer > 4 then
                me.step, me.step_init = 0x4, 0
            end
        elseif step == 0x4 then
            recover(me, s)
        else
            error(string.format("the bubble's phase %#x reads past its table (0x080ED03C)", step))
        end
    end,
}

-- 0x0800CE7C.
local function rock_cube_setup(navi: Object): Action
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0xB2
    navi.attack_damage = 200
    navi.attack_lockout = 0
    return bug_shots.rock_cube_action
end

-- 0x0800CFD0.
local function bubble_setup(navi: Object): Action
    navi.attack_element = 2
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = 100
    navi.attack_lockout = 0
    return bug_shots.bubble_action
end

bug_shots.rock_cube = new.weapon { id = "megaman/rock-cube", charge_ticks = ROCK_CUBE_CHARGE, setup = rock_cube_setup }
bug_shots.bubble = new.weapon { id = "megaman/bubble", charge_ticks = BUBBLE_CHARGE, setup = bubble_setup }

return bug_shots
