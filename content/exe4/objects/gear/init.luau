--!strict
-- EXE4's gear (attack object #0x76, 0x080DBD94): what its gear stages (the actor lists' entry type 5, 0x080068C0:
-- records 32 to 35, two on the metal fields) roll along a row, back and forth on its panel's side, 50 HP, hitting for
-- 20 what it runs into. Its code is EXE6's `sub_80D385C` by its place in the attack table; what it does is read from
-- EXE4's.
--
-- Its mode (its first parameter) picks its side and what it is: 0 (a stage's, 0x080068C0) rolls on its panel's side
-- for the other side; 1 (a lifetime, a field object of its side's registry) on the side it isn't; 2 (an actor's,
-- 0x080C7C74, with a slot its spawner watches) is another kind of body (collision rows 55 and 20). Effect #0x49
-- (0x080E7244, MetlGer's) passes its own first parameter, 1. The stages' spawner and MetlGer's (`gear.spawn`) are
-- ported; the actor's (0x080C7C74) has no user yet.

local collision = require("../rules/collision")

local gear = {}

export type State = {
    -- Its first parameter (0x080DBDE4).
    mode: number,
    -- The side whose panels it rolls on (its second parameter).
    travel: number,
}

local SPRITE = asset.sprite("gear")
local ROLL = asset.sound("gear")
-- Its breaking (0x080DBF32: 0x6F) and effect 0 (the explosion: 14-00's animation 0).
local BROKEN = asset.sound("hit-bomb-0")
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local HP = 50
-- Its damage word (0x080068D0: 20, its hit parameter 0x94).
local DAMAGE = 0x940014
-- Its speed, 16.16 a tick: half a pixel.
local SPEED = 0x8000
local HIT_MOD = 3
-- What its timer starts at (a mode-1 gear's lifetime).
local TIMER = 0x708
-- A panel of side 1's.
local SIDE_1 = 0x20
-- What it rolls onto: a solid panel without a body (0x080DBFB4: `object_checkPanelParameters(0x10, 0x03800000)`).
local SOLID, BODIES = 0x10, 0x03800000

-- Its actions in the framework's numbering (its table's 0, 2 and 6).
local WAIT, DESTROYED, ROLLING = 0, 2, 8

local function state(me: Object): State
    return me.state :: State
end

-- The panel the field position (x, y) is over (`sub_800E258`: the BIOS divisions).
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `sub_800E6E8`: whether a move from `old` to `new` passed `target`.
local function passed(new: number, old: number, target: number): boolean
    if new > old then
        return target > old and target <= new
    end
    return target > new and target <= old
end

-- 0x080DBEB2 and the end of 0x080DBF24: hidden, off the field, out of the registry (mode 1), and gone.
local function leave(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    if state(me).mode == 1 then
        obstacle.unregister(me)
    end
    me:set_lifecycle("destroy")
end

-- 0x080DBEF8: once the fight is on, it no longer runs while paused, shows, and rolls.
local function wait(me: Object)
    if not battle.fighting() then
        return
    end
    me.run_while_paused = false
    me.visible = true
    me:set_action(obstacle.action_byte(me, ROLLING))
end

-- 0x0801013A: back to rolling, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

-- 0x080DBF24: an explosion, a sound, and it goes.
local function destroyed(me: Object)
    battle.effect(me.pos, EXPLOSION)
    battle.play_sound(BROKEN)
    leave(me)
end

-- 0x080DBF5E: its first tick a sound, and the next panel at once; then, past its panel's center, the next panel
-- along its way, else the one under where it is going. A panel it may roll onto (on the field, solid, no body, its
-- travel side's) it rolls on toward; any other turns it back.
local function rolling(me: Object)
    local x, y
    if me.phase_init == 0 then
        battle.play_sound(ROLL)
        me.phase_init = 4
        x, y = me.panel_x + (if me.vel.x >= 0 then 1 else -1), me.panel_y
    else
        local cx = field.center(me.panel_x, me.panel_y)
        local p, v = me.pos, me.vel
        if passed(p.x + v.x, p.x, cx) then
            x, y = me.panel_x + (if v.x >= 0 then 1 else -1), me.panel_y
        else
            x, y = panel_at(p.x + v.x, p.y)
        end
    end
    local side = if bit32.btest(field.flags(x, y), SIDE_1) then 1 else 0
    if field.valid(x, y) and field.check(x, y, SOLID, BODIES) and side == state(me).travel then
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
        me:set_panel_from_coordinates()
        me:set_collision_panel()
    else
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
    end
end

-- Its action table (0x080DBEDC), by the framework's numbers.
local ACTIONS: { [number]: (Object) -> () } = {
    [WAIT] = wait,
    [1] = return_to_idle,
    [DESTROYED] = destroyed,
    [3] = function(_me: Object) end,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [ROLLING] = rolling,
}

-- 0x080DBE6A: hits (never pushed); over the battle, it goes; a mode-1 gear's lifetime; the reaction and its action
-- (0x0801427A, EXE6's `sub_801B878` without LilBoiler's eruption test: destroyed by a touch, the HP as it is; like
-- EXE4's other five reactions it doesn't read the removal request); its sprite, its panel highlighted while it shows,
-- facing its way, and its collision (even on the tick it broke: its region is gone then).
local function tick(me: Object)
    obstacle.take_hits(me, "ignored")
    if battle.over() then
        return leave(me)
    end
    if state(me).mode == 1 then
        obstacle.tick_lifetime(me)
    end
    local a = obstacle.react(me, "destroys")
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("gear action %d reads past its table", a))
        end
        action(me)
    end
    -- (A spawner's slot, 0x080C7C74's, would end it here when cleared, and it clears the slot as it goes: 0x080DBE88,
    -- 0x080DBEC8; a stage's gear has none.)
    me:update_sprite()
    if me.visible then
        field.highlight(me.panel_x, me.panel_y)
    end
    me.sprite:set_flip(if me.vel.x > 0 then 1 else 0)
    me.collision:present()
end

-- 0x080DBDB4: on its panel, on the ground; its side and its way by its mode; its collision made (presented by its
-- first tick).
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.visible = true
    local s = state(me)
    local panel_side = if bit32.btest(field.flags(me.panel_x, me.panel_y), SIDE_1) then 1 else 0
    if s.mode == 0 then
        s.travel = panel_side
        me.alliance = bit32.bxor(panel_side, 1)
    elseif s.mode == 1 then
        s.travel = bit32.bxor(me.alliance, 1)
    else
        s.travel = panel_side
    end
    me.vel = Vec3.new(if s.travel ~= 0 then SPEED else -SPEED, 0, 0)
    me.hp, me.max_hp = HP, HP
    if not me:create_collision() then
        me:free()
        return
    end
    if s.mode == 2 then
        me:setup_collision(collision.collision_37, collision.hit_by_other_side, HIT_MOD)
    else
        me:setup_collision(collision.neutral_body, collision.everything, HIT_MOD)
    end
    -- (Its status word's bit 0, 0x080DBE5C: the guard's.)
    me:set_status("guard", true)
    me.timer = TIMER
    me.lifecycle = "update"
end

gear.kind = new.kind {
    id = "gear",
    pool = "attack",
    state = { mode = "u8", travel = "u8" },
    -- 0x080DBD94.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
    -- 0x080068C0: a gear stage's gear (an actor list's entry type 5), mode 0, running while the battle is paused or
    -- dimmed. (0x080DC006: until its init places it, its position is the spawner's registers, the panel; a mode-1
    -- gear its spawner would register as its side's field object, class 1.)
    place = function(spec: PlaceSpec): Object?
        local o = battle.spawn(gear.kind, Vec3.new(spec.panel_x, spec.panel_y, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o:set_damage_word(DAMAGE)
        o.run_while_paused = true
        o.run_while_dimmed = true
        return o
    end,
}

-- 0x080DC006: a gear on (x, y) of `alliance`'s, of `element` and the damage word, in `mode` (its first parameter),
-- running while the battle is paused or dimmed; a lifetime gear (mode 1) one of its side's field objects (class 1).
-- (Its position is the spawner's registers until its init places it: the row, the element and the side. MetlGer's
-- controller watches no slot: its +0x60 stays empty.)
function gear.spawn(x: number, y: number, element: number, alliance: number, mode: number, damage: number): Object?
    local o = battle.spawn(gear.kind, Vec3.new(y, element, alliance))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance = alliance
    o:set_damage_word(damage)
    o.run_while_paused = true
    o.run_while_dimmed = true
    state(o).mode = mode
    if mode == 1 then
        obstacle.register(o, alliance, 1)
    end
    return o
end

return gear
