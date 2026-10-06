--!strict
-- MagnetSoul's charged shot (`magnetsoul/charge`): the original's weapon routine 0x15 (0x0800F804) sets an Elec attack
-- of 30 damage and 10 more per point of buster Attack plus one (0x0800FE5E), its own counter byte and the parameter
-- that makes its magnet paralyze, and starts action 0x42 (0x080EEB9E): MegaMan raises his magnet (the counter window
-- open), and seven ticks on its field (./magnet) sits on the panel ahead of him for 30 ticks while, each of those
-- ticks, a pull reaches the six panels ahead (region 8, no damage, hit modifier 4: what it hits slides toward him);
-- its hum every 16 ticks. Then 11 ticks of recovery, the magnet let go.

local collision = require("../../../../rules/collision")
local magnet = require("./magnet")
local regions = require("@exelib/regions")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x12: ticks toward the hum.
    hum: number,
}

local HIT_PARAM = 0x94
local ANIM = 16
local RAISE_TICKS, PULL_TICKS, RECOVER_TICKS = 6, 30, 10
local HUM_EVERY = 16
-- The pull's hit modifier: the push of bit 2 (its first row: toward the puller, over the panels).
local PULL_MOD = 4
local SOUND_MAGNET, SOUND_HUM = asset.sound("magnet-pull"), asset.sound("erase-man-2")

-- 0x080EEBBC: the magnet raised; seven ticks on, its field and the pull.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(ANIM)
        me:open_counter_window()
        s.timer = RAISE_TICKS
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t > 0 then
        return
    end
    s.timer = PULL_TICKS
    magnet.spawn(me, me.facing, 0, me.attack_element, true, int.u32(me.attack_damage + me.hit_param * 0x10000))
    battle.play_sound(SOUND_MAGNET)
    battle.play_sound(SOUND_HUM)
    s.hum = 0
    me.step, me.step_init = 4, 0
end

-- 0x080EEC1E: each tick the pull; the hum every 16 ticks; 31 ticks.
local function pull(me: Object, s: State)
    s.hum += 1
    if s.hum == HUM_EVERY then
        s.hum = 0
        battle.play_sound(SOUND_HUM)
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.step, me.step_init = 8, 0
        return
    end
    battle.hitbox(me, {
        panel_x = me.panel_x + me.facing,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.row,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.pull,
        damage = 0,
        stamina = 0,
        hit_mod = PULL_MOD,
    })
end

-- 0x080EEC80: 11 ticks; then the magnet let go and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:set_status("using_action", false)
        me.related1 = nil
        me:exit_attack()
    end
end

charge.action = define.action {
    id = "magnetsoul/charge/action",
    state = { timer = "u16", hum = "u16" },
    -- 0x080EEB9E
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            raise(me, s)
        elseif step == 4 then
            pull(me, s)
        elseif step == 8 then
            recover(me, s)
        else
            error(string.format("MagnetSoul's charged shot's step %#x reads past its table (0x080EEBB0)", step))
        end
    end,
}

-- 0x0800F804 (its parameters' first byte 1, which the action passes on: the magnet paralyzes).
local function setup(navi: Object): Action
    navi.attack_damage = 30 + 10 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 3
    return charge.action
end

charge.weapon = define.weapon {
    -- The charge table's row 0x15 (0x0801CB3E); Charge 5 reads on into row 0x16's first entry.
    charge_ticks = { 90, 85, 80, 75, 70, 80 },
    setup = setup,
}

return charge
