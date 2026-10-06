--!strict
-- ToadMan's charged attack (`toadman/charge`): the original's weapon routine 0x2E (0x0800FA1C) sets the damage his
-- level gives (the damage rows' row 22, lib/navi_level), his own counter byte and the attack's variant 0, and starts
-- his action 8 (0x080F19D8). He dives: 4 ticks going down, then under with a splash, his own panel kept for him and
-- himself off the field for 30 ticks. Then he looks for where to come up, column by column ahead of his panel, each
-- from the top row down: before the first body of the other side's with a free panel that can be stood on between it
-- and him. There he surfaces with a splash, rising 8 pixels on the first tick, and strikes: a 60-tick counter
-- window, and on his 9th tick a hit on the panel ahead (an Aqua attack at navis, 16 pixels up, hit modifier 3). 13
-- ticks, 5 more, 4 going down, and he is back on his own panel: 4 ticks coming up and 20 standing. With nowhere to
-- come up he comes straight back.
--
-- The attack's variant 1 comes up before a target it is given (0x080F1CA0): his auto battle's, which nothing a
-- player does sets.

local collision = require("../../rules/collision")
local navi_level = require("../../lib/navi_level")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the current step's ticks (counted down, the strike's up).
    timer: number,
}

local HIT_PARAM = 0x94
-- The damage rows' row 22, by his level.
local DAMAGE: { number } = { 100, 100, 100, 100, 100, 100, 100 }
local SPLASH = new.effect { sprite = asset.sprite("dive-splash"), anim = 0, palette = 0 }
local SOUND_DIVE = asset.sound("toad-dive")
local SOUND_SURFACE = asset.sound("sand-worm")
-- His animations: going down, coming up at home, surfacing, striking.
local ANIM_DOWN, ANIM_UP, ANIM_SURFACE, ANIM_STRIKE = 4, 3, 18, 19
local DOWN_TICKS, UNDER_TICKS, SURFACE_TICKS, STRIKE_AT, STRIKE_TICKS, HOLD_TICKS, UP_TICKS, STAND_TICKS = 4, 30, 2, 8, 12, 5, 4, 20
local COUNTER_TICKS = 60
-- What he rises a tick as he surfaces, and how high his strike hits (16.16).
local RISE, STRIKE_Z = 8 * 0x1_0000, 16 * 0x1_0000
local AQUA = 2
local HIT_MOD = 3
-- The panel flags (0x080F1C98, 0x080F1C68): a body of the other side's, by his side; a panel that can be stood on;
-- any body.
local ENEMY_BODY: { [number]: number } = { [0] = 0x0400_0000, 0x0800_0000 }
local STANDS, BODY = 0x10, 0x0F80_0000

-- The steps, on the attack's step (the table's entry 4 is the step at 8, which nothing goes to).
local DIVE, UNDER, SURFACE, STRIKE, HOLD, DOWN, HOME, STAND = 0x0, 0x8, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20

-- Count the step's ticks down; true once they have run out.
local function over(s: State): boolean
    local t = s.timer - 1
    s.timer = int.u16(t)
    return t <= 0
end

-- 0x080F1C20: where he comes up: from the column ahead of his own panel on, each column from the top row down,
-- the panel before the first body of the other side's that can be stood on and has no body on it; none once the
-- columns run out.
local function surfacing(me: Object): (number?, number?)
    local front = me.facing
    local x = me.future_panel_x + front
    while field.valid(x, 1) do
        for y = 1, 3 do
            if field.check(x, y, ENEMY_BODY[me.alliance], 0) and field.check(x - front, y, STANDS, BODY) then
                return x - front, y
            end
        end
        x += front
    end
    return nil, nil
end

-- 0x080F1A10: going down, 4 ticks, a move as the engine's steps are; a navi that can't move goes straight to the
-- step home.
local function dive(me: Object, s: State)
    if me.step_init == 0 then
        if not me:can_move() then
            me.step, me.step_init = HOME, 0
            return
        end
        me:set_status("moving", true)
        me:set_animation(ANIM_DOWN)
        s.timer = DOWN_TICKS
        me.step_init = 4
    end
    if over(s) then
        me.step, me.step_init = UNDER, 0
    end
end

-- 0x080F1A48: under: the splash where he was, his panel kept for him, himself off the field with no hit region;
-- 30 ticks.
local function under(me: Object, s: State)
    if me.step_init == 0 then
        battle.effect(me.pos, SPLASH)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me.panel_x, me.panel_y = 0, 0
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me.collision:set_region(nil)
        battle.play_sound(SOUND_DIVE)
        s.timer = UNDER_TICKS
        me.step_init = 4
    end
    if over(s) then
        me.step, me.step_init = SURFACE, 0
    end
end

-- 0x080F1A9C: his hit region back; where he comes up, or home with nowhere; there, the splash and its sound, and he
-- rises for a tick.
local function surface(me: Object, s: State)
    if me.step_init == 0 then
        me.collision:set_region(regions.single)
        local x, y = surfacing(me)
        if not x or not y then
            me.step, me.step_init = HOME, 0
            return
        end
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_animation(ANIM_SURFACE)
        me:set_status("using_action", true)
        s.timer = SURFACE_TICKS
        battle.effect(me.pos, SPLASH)
        battle.play_sound(SOUND_SURFACE)
        me.step_init = 4
    end
    if over(s) then
        me.step, me.step_init = STRIKE, 0
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, int.i32(p.z + RISE))
end

-- 0x080F1B24: the strike: on the ground, the counter window open; the hit ahead when 8 ticks have gone; 13 ticks.
local function strike(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM_STRIKE)
        me:open_counter_window(COUNTER_TICKS)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == STRIKE_AT then
        battle.hitbox(me, {
            panel_x = me.panel_x + me.facing,
            panel_y = me.panel_y,
            element = AQUA,
            z = STRIKE_Z,
            region = regions.single,
            hit_effect = sparks.aqua,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = HIT_MOD,
        })
    end
    s.timer = int.u16(s.timer + 1)
    if s.timer > STRIKE_TICKS then
        me.step, me.step_init = HOLD, 0
    end
end

-- 0x080F1B7C: 5 ticks more.
local function hold(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = HOLD_TICKS
        me.step_init = 4
    end
    if over(s) then
        me.step, me.step_init = DOWN, 0
    end
end

-- 0x080F1B9A: going down again, 4 ticks.
local function down(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM_DOWN)
        s.timer = DOWN_TICKS
        me.step_init = 4
    end
    if over(s) then
        me.step, me.step_init = HOME, 0
    end
end

-- 0x080F1BBE: back on his own panel, let go; coming up, 4 ticks.
local function home(me: Object, s: State)
    if me.step_init == 0 then
        local x, y = me.future_panel_x, me.future_panel_y
        me.panel_x, me.panel_y = x, y
        me:unreserve_panel(x, y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_animation(ANIM_UP)
        s.timer = UP_TICKS
        me.step_init = 4
    end
    if over(s) then
        me.step, me.step_init = STAND, 0
    end
end

-- 0x080F1BF6: the move over; 20 ticks standing; then back to idle.
local function stand(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("moving", false)
        me:set_animation(0)
        s.timer = STAND_TICKS
        me.step_init = 4
    end
    if over(s) then
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "toadman/charge/action",
    state = { timer = "u16" },
    -- 0x080F19D8
    update = function(me: Object, s: State)
        local step = me.step
        if step == DIVE then
            dive(me, s)
        elseif step == UNDER or step == 0x4 then
            under(me, s)
        elseif step == SURFACE then
            surface(me, s)
        elseif step == STRIKE then
            strike(me, s)
        elseif step == HOLD then
            hold(me, s)
        elseif step == DOWN then
            down(me, s)
        elseif step == HOME then
            home(me, s)
        elseif step == STAND then
            stand(me, s)
        else
            error(string.format("ToadMan's charged attack's step %#x reads past its table (0x080F19EC)", step))
        end
    end,
}

-- 0x0800FA1C
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_variant = 0
    return charge.action
end

charge.weapon = new.weapon {
    id = "toadman/charge",
    -- The charge table's row 0x2E (0x0801CC38), by Charge 0 to 5: past 4 it reads on into row 0x2F's first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 30 },
    setup = setup,
}

return charge
