--!strict
-- TomahawkCross Beast's charged Null chip (`tomahawkcross-beast/throw`):
-- the original's weapon routine 0x1B (`sub_8011DBC`) and action 0x4E
-- (`sub_80EF1EC`). The navi winds up;
-- when the wind-up animation ends it swings, and two tomahawks
-- (objects/boomerang, its tomahawk variant) fly from its side's back
-- column, one along row 1 and 10 ticks later one along row 3; 96 ticks
-- after the swing began it idles.

local boomerang = require("../../../../objects/boomerang/boomerang")

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10).
    timer: number,
}

local throw = {}

-- The phases, on the attack's step.
local WIND_UP, SWING = 0x0, 0x4

-- Wood, and the hit's counter byte.
local WOOD, HIT_PARAM = 4, 0x9E

-- The swing's length, and when the tomahawks fly (on row 1, then row 3).
local SWING_TICKS = 0x5F
local ROW_1_AT, ROW_3_AT = 0x5D, 0x53

-- `sub_8011DBC`: 50 damage and 20 more per point of buster Attack (up to
-- 5) (`sub_8012642`), Wood.
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = 0x32 + 0x14 * math.min(navi:buster_damage(), 5)
    navi.attack_element = WOOD
    return throw.action
end

-- `sub_80EF208`: the wind-up, until its animation's last frame.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x17)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        me.step_init = 4
        return
    end
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me:set_animation(0x18)
    s.timer = SWING_TICKS
    me.step = SWING
    me.step_init = 0
end

-- `sub_80EF248`: the swing; the tomahawks fly from the side's back column
-- (whichever way the navi faces).
local function swing(me: Object, s: State)
    local t = s.timer
    local row = if t == ROW_1_AT then 1 elseif t == ROW_3_AT then 3 else nil
    if row then
        local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
        local x = me.alliance * 5 + 1
        boomerang.spawn(me, x, row, me.attack_element, 0x10_0000, { variant = boomerang.tomahawk }, damage)
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me:set_status("moving", false)
        me:exit_attack()
    end
end

throw.action = define.action {
    id = "tomahawkcross-beast/throw/action",
    state = { timer = "u16" },
    -- `sub_80EF1EC`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == SWING then
            swing(me, s)
        else
            error(string.format("tomahawk throw phase %#x reads past its phase table", step))
        end
    end,
}

throw.weapon = define.weapon {
    id = "tomahawkcross-beast/throw",
    name = "Tomahawk throw",
    -- The charge table's row 0x1B, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 40, 40, 40, 40, 40, 20, 20, 20 },
    setup = setup,
}

return throw
