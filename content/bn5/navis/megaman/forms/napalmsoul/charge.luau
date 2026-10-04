--!strict
-- NapalmSoul's charged shot (`napalmsoul/charge`): the original's weapon routine 0x19 (0x0800F848) sets a Fire attack
-- of 5 damage and 5 more per point of buster Attack plus one (0x0800FE5E), its own counter byte and no bonus, and
-- starts action 0x44 (0x080EED9A): MegaMan raises his cannon (the counter window open) and fires three fire bullets
-- from the panel ahead, eleven ticks apart, each at a random height (0x080EEE5C); the last flinches. Then 11 ticks of
-- recovery.

local bullet = require("../../../../objects/bullet/bullet")
local bullets = require("@exelib/bullet/bullet")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x12: shots left.
    shots: number,
}

local HIT_PARAM = 0x8A
local SHOTS = 3
-- The bullets' heights, by a draw's low two bits (pixels).
local HEIGHTS = { [0] = 8, 16, 24, 32 }
local SOUND = asset.sound("col-army-2")
local RAISE_ANIM, FIRE_ANIM = 16, 17
local RAISE_TICKS, BETWEEN, RECOVER_TICKS = 1, 10, 10

-- 0x080EEDB8: the cannon raised; two ticks.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(RAISE_ANIM)
        me:open_counter_window()
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = RAISE_TICKS
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EEDE8: a bullet at once and every eleven ticks; after the third, the recovery.
local function fire(me: Object, s: State)
    if me.step_init == 0 then
        s.shots = SHOTS
        me:set_animation(FIRE_ANIM)
        s.timer = 0
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t >= 0 then
        return
    end
    s.timer = BETWEEN
    local variant = if s.shots > 1 then bullet.variants.napalm else bullet.variants.napalm_last
    local z = HEIGHTS[bit32.band(battle.rng_positive(), 3)] * 0x1_0000
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    bullets.fire(bullet.kind, me, me.panel_x + me.facing, me.panel_y, z, variant, me.attack_element, damage)
    battle.play_sound(SOUND)
    s.shots -= 1
    if s.shots <= 0 then
        me.step, me.step_init = 8, 0
    end
end

-- 0x080EEE60: 11 ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(RAISE_ANIM)
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
    id = "napalmsoul/charge/action",
    state = { timer = "u16", shots = "u16" },
    -- 0x080EED9A
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            raise(me, s)
        elseif step == 4 then
            fire(me, s)
        elseif step == 8 then
            recover(me, s)
        else
            error(string.format("NapalmSoul's charged shot's step %#x reads past its table (0x080EEDAC)", step))
        end
    end,
}

-- 0x0800F848 (its shots count, AIAttackVars +0x12, the action sets again).
local function setup(navi: Object): Action
    navi.attack_element = 1
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = 5 + 5 * (battle.navi(navi.alliance).attack + 1)
    return charge.action
end

charge.weapon = define.weapon {
    id = "napalmsoul/charge",
    -- The charge table's row 0x19 (0x0801CB66); Charge 5 reads on into row 0x1A's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 160 },
    setup = setup,
}

return charge
