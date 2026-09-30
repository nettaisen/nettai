--!strict
-- TomahawkMan's own chip, ETomahwk (action 0x0A for AI index 7,
-- `sub_80F18AC`): he raises his axe (effect object #0x32,
-- objects/eagle-tomahawk) for 17 ticks, swings it, and 10 ticks in the
-- blow falls: a tomahawk strike (attack object #0x6A,
-- objects/tomahawk-strike) on each panel ahead to the field's edge, up to
-- six, 5 ticks apart, each breaking its panel. 30 ticks later the axe goes
-- and he recovers for 30. [unverified: no lab scenario reaches it;
-- docs/engine/standard-chips.md]

local eagle_tomahawk = require("../../objects/eagle-tomahawk/eagle_tomahawk")
local tomahawk_strike = require("../../objects/tomahawk-strike/tomahawk_strike")
local link = require("../190-heatpres/state")
type State = LinkChipState

local routine = {}

local RAISE, SWING, RECOVER = 0x0, 0x4, 0x8
-- The axe kind for the link chip (it lasts through action 0xA).
local AXE = 1
-- The strikes: 5 ticks apart, self type 7, target 5, no spark.
local DELAY, SELF, TARGET, NO_SPARK = 5, 7, 5, 0xFF
-- The tick (timer value) the blow falls.
local FALL_AT = 0x14

-- `sub_80F18CC`: the axe out; 17 ticks.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = 0x11
        s.held = eagle_tomahawk.spawn(me, AXE)
        s.timer = 0x10
    elseif link.count_down(s) then
        link.go(me, SWING)
    end
end

-- `sub_80F197A`: a strike on each panel ahead, while on the field.
local function strikes(me: Object)
    for n = 1, 6 do
        local x, y = me.panel_x + me.facing * n, me.panel_y
        if not field.valid(x, y) then
            return
        end
        local params = { (n - 1) * DELAY, SELF, TARGET, NO_SPARK }
        tomahawk_strike.spawn(me, x, y, me.attack_element, params, link.damage_word(me), int.i32(0xFF00_0000))
    end
end

-- `sub_80F1902`: the swing; the blow falls 10 ticks in with a camera shake
-- (magnitude 2, 30 ticks), sound 0x10C; 30 ticks.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0x12
        s.timer = 0x1E
        local axe = s.held
        if axe then
            axe.action = 1
        end
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == FALL_AT then
        battle.shake_camera(2, 0x1E)
        battle.play_sound(0x10C)
        strikes(me)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F194C`: the axe goes; 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 7
        s.timer = 0x1E
        local axe = s.held
        if axe then
            axe.action = 2
        end
    elseif link.count_down(s) then
        me:exit_attack()
    end
end

-- `sub_80F18AC` (`off_80F18C0`).
function routine.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == SWING then
        swing(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("ETomahwk phase %#x reads past off_80F18C0", step))
    end
end

return routine
