--!strict
-- BN5's SideBub1-3 (action 0x1C, 0x080EC832): BN5's own. The navi raises its buster arm and, on the action's
-- second tick, two bubbles (attack object #0x8F, 0x080D8484) rise on the column ahead, one in the row above its own
-- (its own, from the top row) and one in the row below (its own, from the bottom row). Each waits 16 ticks, then
-- drifts a panel forward and a row up or down every 31 ticks (the top row's down, the bottom row's up, the middle
-- row's as its spawner says: the upper bubble's up, the lower's down), turning back at the top and bottom rows.
-- A bubble hits the other side's navis and objects with the chip's damage and pops on the first it touches; an
-- attack that pops it (it has 1 HP) bursts it, and 20 ticks later it sends an aqua shot forward (BN5's flying shot
-- row 7) with the chip's damage.

local arm_launch = require("../../lib/arm-launch")
local collision = require("../../rules/collision")
local flying_shot = require("../../objects/flying-shot/flying_shot")
local sparks = require("../../lib/sparks")

local sidebub = {}

export type State = {
    -- Which way it drifts at first in the middle row (its first parameter: 1 up, 2 down, 0 at random).
    way: number,
    -- The ticks a drift lasts (its second parameter: the chip's second parameter byte, 30 for each SideBub).
    travel: number,
    -- Which row of BN5's flying shot it bursts into (its fourth parameter, read through 0x080D86B4).
    burst: number,
}

local SPRITE = asset.sprite("sprite-10-41")
local SOUND_RISE = asset.sound("sound-123")
local SOUND_POP = asset.sound("bubble-pop")
local SOUND_BURST = asset.sound("junk-shot")
local HIT_MOD = 3
-- Its waits: before the first drift (with the animation set to 0 at 5 left), and before its burst shot.
local WAIT, WAIT_ANIM_AT, BURST_WAIT = 15, 5, 20
-- How far a drift goes: a panel forward, a row up or down (in pixels).
local DRIFT_X, DRIFT_Y = 40, 24
-- The rows of BN5's flying shot a bubble's fourth parameter picks (0x080D86B4).
local BURSTS: { [number]: number } = { [0] = 7, [1] = 8, [2] = 9, [3] = 9 }
-- What a hit on a navi or an object leaves in its hit flags (0x080D85B4).
local POPS = 0x0F80_0000
-- Its actions.
local WAITING, DRIFTING, BURSTING = 0x0, 0x4, 0x8

-- 0x080D8620: the first wait.
local function waiting(me: Object, s: State)
    if me.timer == WAIT_ANIM_AT then
        me.anim = 0
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me.timer = s.travel
        me.action = DRIFTING
    end
end

-- 0x080D863E: drifting; halfway, onto the next panel; at the end, set on it, and back the other way unless in the
-- middle row.
local function drifting(me: Object, s: State)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), p.z)
    if me.timer == me.timer2 then
        me:set_panel_from_coordinates()
        me:update_collision_panels()
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me.timer = s.travel
        if me.panel_y ~= 2 then
            me.vel = Vec3.new(v.x, int.i32(-v.y), v.z)
        end
    end
end

-- 0x080D8682: the burst; 20 ticks on, the shot from where it is, and gone.
local function bursting(me: Object, s: State)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me.collision:set_region(nil)
        flying_shot.launch(me, me.pos, s.burst, int.u32(me.damage + me.stamina * 0x10000))
        battle.play_sound(SOUND_BURST)
        me:set_lifecycle("destroy")
    end
end

local function gone(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D858E: the hits against it resolve and spark; when the battle is over, gone; an attack that empties its HP
-- bursts it (its body becomes an attack: BN5's 0x08016B9E keeps what it hits, below); a navi or an object it hits
-- pops it; while the battle isn't dimmed, its action; off the field, gone.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        gone(me)
        return
    end
    local s = me.state :: State
    if me.action ~= BURSTING and me:take_damage(0) == -1 then
        battle.play_sound(SOUND_POP)
        c:set_region(nil)
        me.anim = 1
        -- (0x08016B9E: its store of what it hits goes astray, so it keeps hitting what it hit: BN5's retype rule.)
        me:reset_collision_types(collision.attack, collision.hits_navis, HIT_MOD)
        me.timer = BURST_WAIT
        me.action = BURSTING
        return
    end
    if bit32.btest(c.hit_flags, POPS) then
        battle.play_sound(SOUND_POP)
        gone(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == WAITING then
            waiting(me, s)
        elseif a == DRIFTING then
            drifting(me, s)
        elseif a == BURSTING then
            bursting(me, s)
        else
            error(string.format("SideBub's bubble's action %#x reads past 0x080D8614", a))
        end
    end
    if not field.valid(me.panel_x, me.panel_y) then
        gone(me)
        return
    end
    c:present()
end

-- 0x080D84BE: its look (sheet 10-41, with a shadow, animation 3) and 1 HP on its panel; its collision (an object of
-- its side hit by the other side's attacks and hitting the other side's navis and objects, hit modifier 3, the aqua
-- spark); its drift's speed and first way; the sound.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 3, 3
    sprite:set_animation(3)
    me.hp, me.max_hp = 1, 1
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_object, collision.hit_by_other_side_or_blockers, HIT_MOD)
    -- (Its region again: its own panel, as the setup left it.)
    local c = me.collision
    c:set_hit_effect(sparks.aqua)
    c:present()
    local way: number
    if me.panel_y == 3 then
        way = -1
    elseif me.panel_y == 1 then
        way = 1
    else
        local w = s.way
        if w == 0 then
            w = bit32.band(battle.rng_positive(), 2)
        end
        way = bit32.band(w, bit32.bnot(1)) - 1
    end
    local travel = s.travel
    if travel == 0 then
        error("SideBub's bubble drifts no ticks: 0x080D8548 divides by zero")
    end
    me.vel = Vec3.new(
        int.tdiv(DRIFT_X * 0x10000, travel) * me.facing,
        int.tdiv(DRIFT_Y * 0x10000, travel) * way,
        me.vel.z
    )
    me.timer2 = bit32.rshift(travel, 1)
    me.timer = WAIT
    battle.play_sound(SOUND_RISE)
    me:set_lifecycle("update")
    tick(me)
end

sidebub.bubble = define.kind {
    id = "sidebub/bubble",
    pool = "attack",
    state = { way = "u8", travel = "u8", burst = "u8" },
    -- 0x080D8484.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_paused()
    end,
}

-- 0x080D86BA: a bubble of `owner`'s on (x, y) with the element, the damage word and its parameters (its first way,
-- its drift's ticks, its burst's row by 0x080D86B4); it runs while the battle is dimmed. (Its X and Y are the
-- spawner's registers, the panel's Y and the element, until its init places it.)
function sidebub.spawn(owner: Object, x: number, y: number, element: number, damage: number, way: number, travel: number, burst: number): Object?
    local row = BURSTS[burst]
    if not row then
        error(string.format("SideBub's bubble's burst %d reads past 0x080D86B4", burst))
    end
    local o = battle.spawn(sidebub.bubble, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        local s = o.state :: State
        s.way, s.travel, s.burst = way, travel, row
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- The chip's parameters (AIAttackVars+0x0C, its record's +0x10): the drift's ticks and the burst (30 and 0 for each
-- SideBub).
local TRAVEL, BURST = 30, 0

-- 0x080EC84C: the arm raised; on the action's second tick the two bubbles (lib/arm-launch).
sidebub.action = arm_launch.action("0x080EC844", function(me: Object, damage: number)
    local x = me.panel_x + me.facing
    local up = me.panel_y - 1
    if up == 0 then
        up = 1
    end
    sidebub.spawn(me, x, up, me.attack_element, damage, 1, TRAVEL, BURST)
    local down = me.panel_y + 1
    if down >= 4 then
        down = 3
    end
    sidebub.spawn(me, x, down, me.attack_element, damage, 2, TRAVEL, BURST)
end)

return sidebub
