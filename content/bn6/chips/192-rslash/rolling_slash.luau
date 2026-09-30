--!strict
-- SlashMan's own chip, RSlash (action 0x0A for AI index 3, `sub_80F0CB0`):
-- he curls up (21 ticks), then rolls around the field at 4 pixels a tick
-- with a riding hit on him (attack object #0x69, objects/riding-hit): from
-- the bottom row straight at the enemy, else down to the bottom row first,
-- then along the route of `byte_80F0F50` (across, up the far column, back
-- along the top row, down his own column), turning at each leg's column or
-- row, until he is home, runs onto a panel that isn't solid or holds a
-- neutral object, or (on the first leg) finds an enemy in his column. Then
-- he lands on his panel. The leg he is on counts in the attack's
-- parameter word. [unverified: no lab scenario reaches it;
-- docs/engine/standard-chips.md]

local riding_hit = require("../../objects/riding-hit/riding_hit")
local link = require("../190-heatpres/state")
type State = LinkChipState

local routine = {}

local CURL, LAUNCH, ROLL, LAND = 0x0, 0x4, 0x8, 0xC
local SPEED = 0x4_0000
-- The riding hit: self type 7, target 5; hit modifier 3, no spark, keeps
-- going after a hit.
local HIT_SELF, HIT_TARGET, HIT_PARAMS = 7, 5, { 3, 0xFF, 1 }
-- What he rolls onto: solid, with flag 0x10000, no neutral object
-- (`dword_80F0DE4`).
local ROLL_REQUIRE, ROLL_FORBID = 0x0001_0010, 0x0080_0000
-- The route by side and leg (`byte_80F0F50`): the column or row where the
-- leg ends (0: none), and the direction of the next.
type Leg = { x: number, y: number, dx: number, dy: number }
local ROUTE: { [number]: { [number]: Leg } } = {
    [0] = {
        [0] = { x = 0, y = 3, dx = 1, dy = 0 },
        { x = 6, y = 0, dx = 0, dy = -1 },
        { x = 0, y = 1, dx = -1, dy = 0 },
        { x = 1, y = 0, dx = 0, dy = 1 },
        { x = 0, y = 0, dx = 0, dy = 0 },
    },
    [1] = {
        [0] = { x = 0, y = 3, dx = -1, dy = 0 },
        { x = 1, y = 0, dx = 0, dy = -1 },
        { x = 0, y = 1, dx = 1, dy = 0 },
        { x = 6, y = 0, dx = 0, dy = 1 },
        { x = 0, y = 0, dx = 0, dy = 0 },
    },
}
-- An enemy's body in a column (`off_80F0F98`, by side).
local ENEMY_BODY = { [0] = 0x0400_0000, 0x0800_0000 }

-- The attack's parameter word (AIAttackVars+0x0C): the leg is its low
-- byte, and the game adds 1 to the whole word.
local function leg(me: Object): number
    return me:attack_param(1)
end

local function next_leg(me: Object)
    local word = me:attack_param(1) + me:attack_param(2) * 0x100 + me:attack_param(3) * 0x10000 + me:attack_param(4) * 0x1000000
    word = int.u32(word + 1)
    for i = 1, 4 do
        me:set_attack_param(i, bit32.band(bit32.rshift(word, 8 * (i - 1)), 0xFF))
    end
end

-- `sub_800E258`: the panel at a position (BIOS divisions).
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `sub_800E708`: `c` lies between `a` and `b`, ends included.
local function between(a: number, b: number, c: number): boolean
    if a == c or b == c then
        return true
    end
    if a > b then
        return c >= b and c <= a
    end
    return c > a and c <= b
end

-- `sub_80F0F78`: an enemy stands in his column (`object_getPanelsInColumnFiltered`).
local function enemy_in_column(me: Object): boolean
    for y = 1, 3 do
        if field.check(me.panel_x, y, ENEMY_BODY[me.alliance], 0) then
            return true
        end
    end
    return false
end

-- `sub_80F0EC0`, at a panel's center: turn onto the next leg where this
-- one ends; true once he is back on his own panel.
local function turn(me: Object, s: State): boolean
    local n = leg(me)
    local r = ROUTE[me.alliance][n]
    if r == nil then
        error(string.format("RSlash leg %d reads past byte_80F0F50", n))
    end
    local function go()
        me.vel = Vec3.new(r.dx * s.speed, r.dy * s.speed, me.vel.z)
        next_leg(me)
    end
    local home = me.future_panel_x == me.panel_x
    if n == 1 and enemy_in_column(me) then
        go()
        return false
    end
    if n == 4 then
        -- `loc_80F0F2A`: home once back on his row.
        return me.future_panel_y == me.panel_y
    end
    if n ~= 3 then
        if r.x ~= 0 then
            if me.panel_x == r.x then
                go()
            end
            return false
        elseif r.y ~= 0 then
            if me.panel_y == r.y then
                go()
            end
            return false
        end
    end
    -- `loc_80F0F16`: on his column; home on his row, else turn.
    if not home then
        return false
    end
    if me.future_panel_y ~= me.panel_y then
        go()
        return false
    end
    return true
end

-- `sub_80F0CD4`: curled up for 21 ticks.
local function curl(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        s.held = nil
        s.speed = SPEED
        me.anim = 0x12
        s.timer = 0x14
    elseif link.count_down(s) then
        link.go(me, LAUNCH)
    end
end

-- `sub_80F0E96`: from the bottom row straight at the enemy; else down.
local function head_off(me: Object, s: State)
    if me.panel_y == 3 then
        me.vel = Vec3.new(me.facing * s.speed, 0, me.vel.z)
        me:set_attack_param(1, 1)
    else
        me.vel = Vec3.new(0, s.speed, me.vel.z)
        me:set_attack_param(1, 0)
    end
end

-- `sub_80F0D08`: rolling, the riding hit on, sound 0x164; 11 ticks.
local function launch(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.collision.region = 0
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_status("moving", true)
        me.anim = 0x13
        s.held = riding_hit.spawn(me, HIT_SELF, HIT_TARGET, me.attack_element, HIT_PARAMS, link.damage_word(me))
        battle.play_sound(0x164)
        s.timer = 0x0A
    elseif link.count_down(s) then
        head_off(me, s)
        link.go(me, ROLL)
    end
end

-- `sub_80F0D64`: roll; at each panel's center, turn or stop.
local function roll(me: Object, s: State)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    local across = bit32.btest(leg(me), 1)
    local center = if across then cx else cy
    local old = me.pos
    local v = me.vel
    local x, y = int.i32(old.x + v.x), int.i32(old.y + v.y)
    me.pos = Vec3.new(x, y, old.z)
    local px, py = panel_at(x, y)
    if not field.check(px, py, ROLL_REQUIRE, ROLL_FORBID) then
        link.go(me, LAND)
        return
    end
    local was, now = if across then old.x else old.y, if across then x else y
    if center ~= was and between(now, was, center) then
        me:set_coordinates_from_panel()
        if turn(me, s) then
            link.go(me, LAND)
            return
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80F0DEC`: back on his panel, region 1, the riding hit gone; 20
-- ticks, then idle.
local function land(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 7
        me.collision.region = 1
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_status("moving", false)
        s.timer = 0x14
        local hit = s.held
        if hit then
            riding_hit.stop(hit)
        end
    elseif link.count_down(s) then
        me:exit_attack()
    end
end

-- `sub_80F0CB0` (`off_80F0CC4`).
function routine.update(me: Object, s: State)
    local step = me.step
    if step == CURL then
        curl(me, s)
    elseif step == LAUNCH then
        launch(me, s)
    elseif step == ROLL then
        roll(me, s)
    elseif step == LAND then
        land(me, s)
    else
        error(string.format("RSlash phase %#x reads past off_80F0CC4", step))
    end
end

return routine
