--!strict
-- MeddySoul's charged shot (`meddysoul/charge`): the original's weapon routine 0xE (0x0800F728) sets 30 damage and 10
-- more per point of buster Attack plus one (0x0800FE5E), its own counter byte, no element and the capsule's
-- parameters (variant 0, its column, not running while dimmed), and starts action 0x3E (0x080EEA58): MegaMan throws
-- (the counter window open) and on the tenth tick, with a sound, a capsule (Meddy's, chips/meddy/capsule) at the panel
-- three ahead. 21 ticks, then 6 of recovery.

local capsule = require("../../../../chips/meddy/capsule")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks (counted up, then down).
    timer: number,
}

local HIT_PARAM = 0x94
local THROW_ANIM = 6
local SOUND = asset.sound("bn5:minibomb-throw")
local THROW_AT, THROW_TICKS, RECOVER_TICKS = 9, 21, 5
local REACH = 3

-- 0x080EEA74: the throw; the capsule on the count's 9; 21 ticks.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(THROW_ANIM)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == THROW_AT then
        battle.play_sound(SOUND)
        capsule.throw(me, me.attack_element, {
            variant = 0,
            line = 0,
            target_x = me.panel_x + REACH * me.facing,
            target_y = me.panel_y,
            damage = int.u32(me.attack_damage + me.hit_param * 0x10000),
            mode = 0,
        })
        me.related1 = nil
        me.overlay = nil
    end
    s.timer += 1
    if s.timer >= THROW_TICKS then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EEAD8: 6 ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:exit_attack()
    end
end

charge.action = define.action {
    id = "bn5:meddysoul/charge/action",
    state = { timer = "u16" },
    -- 0x080EEA58
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            throw(me, s)
        elseif step == 4 then
            recover(me, s)
        else
            error(string.format("MeddySoul's charged shot's step %#x reads past its table (0x080EEA6C)", step))
        end
    end,
}

-- 0x0800F728
local function setup(navi: Object): Action
    navi.attack_damage = 30 + 10 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    return charge.action
end

charge.weapon = define.weapon {
    id = "bn5:meddysoul/charge",
    -- The charge table's row 0xE (0x0801CAF8); Charge 5 reads on into row 0xF's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 0 },
    setup = setup,
}

return charge
