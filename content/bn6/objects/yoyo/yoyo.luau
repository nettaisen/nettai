--!strict
-- A yoyo (attack object #0x52, `sub_80CE81C`): YoYo's, and each of
-- GreatYo's three. It rolls out a panel per 8 ticks for its third
-- parameter's panels (5 pixels a tick, re-arming its region on each new
-- panel), spins there for its fourth parameter's 8-tick rounds (its region
-- re-armed each round) and rolls back as far as it went. A hit takes its
-- region away until the next panel or round. It runs while dimmed but
-- only resolves its hits then.
--
-- GreatYo's (second parameter 1) move together: a hit on the way out
-- sends all three into their spin at once (`sub_80E8612`), and the
-- controller (objects/great-yoyo) keeps them. When it goes it clears the
-- slot that holds it: its navi's first related object, or its
-- controller's.

local object_setup = require("../../lib/object_setup")

-- Which slot holds the yoyo (ExtraVars+4, a pointer in the game): its
-- navi's first related object, or one of its GreatYo controller's.
export type YoyoSlot = "related" | "middle" | "below" | "above"

export type State = {
    -- Panels or rounds left (ExtraVars+0).
    left: number,
    slot: YoyoSlot,
    -- 1 once it hit something (and when told to spin by another yoyo; 2
    -- makes it skip its next spin tick) (ExtraVars+8).
    hit: number,
    -- Panels to roll back (ExtraVars+0xC).
    back: number,
}

local yoyo = {
    state = { left = "i32", slot = { "related", "middle", "below", "above" }, hit = "u32", back = "i32" },
}

-- The controller's slots (objects/great-yoyo's state).
type Controller = { middle: Object?, below: Object?, above: Object? }

local SPRITE = "0c-0b"
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 7, 5, 1
-- Its speed, 16.16 per tick, and the ticks per panel or round.
local SPEED, ROUND = 0x5_0000, 8
-- A GreatYo's (second parameter).
local GREAT = 1
-- The actions: out, spinning, back.
local OUT, SPIN, BACK = 0, 4, 8

-- `sub_80CEAA2`: a yoyo for `owner` on (x, y) at height `z`, with the
-- damage word `damage` and the chip's `params`, held in `slot`. It runs
-- while dimmed. Its position is the spawner's registers (the panel Y, the
-- element and the height) until its init places it.
function yoyo.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    params: { number },
    slot: YoyoSlot
): Object?
    local o = battle.spawn_kind("yoyo", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner;
        (o.state :: State).slot = slot
        o.run_while_dimmed = true
    end
    return o
end

-- The controller that holds a GreatYo's yoyo (its first related object).
local function controller(me: Object): Object?
    local owner = me.related1
    if owner == nil then
        return nil
    end
    if owner.pool ~= "effect" or owner.index ~= 0x70 then
        error("a yoyo with its second parameter 1 reads a navi's ExtraVars as a GreatYo controller (sub_80E8612)")
    end
    return owner
end

-- `sub_80CEAC2`: tell `target` to spin (snapped to its panel), with the
-- hit flag `hit` (1 for the caller itself, and every yoyo after it) and
-- `back` panels to roll back. Returns the flag the next yoyo gets.
local function signal(me: Object, target: Object?, hit: number, back: number): number
    if target == me then
        hit = 1
    end
    if target then
        target:set_coordinates_from_panel()
        local s = target.state :: State
        s.hit = hit
        s.back = back
        target:set_action(SPIN)
    end
    return hit
end

-- `sub_80E8612`: all three of the GreatYo's yoyos spin, in the
-- controller's order.
local function all_spin(me: Object, back: number)
    local c = controller(me)
    if c == nil then
        return
    end
    local slots = c.state :: Controller
    local hit = 2
    hit = signal(me, slots.below, hit, back)
    hit = signal(me, slots.above, hit, back)
    signal(me, slots.middle, hit, back)
end

-- `sub_800E258`'s column: the panel under X.
local function column(x: number): number
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28)
end

-- Roll a tick; on a new panel its region is back.
local function roll(me: Object)
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    local col = column(x)
    if col ~= me.panel_x then
        me.panel_x = col
        me.collision.panel_x = col
        me.collision.region = 1
    end
end

-- Out of panels (or a GreatYo's hit): spin, with the panels to roll back.
local function turn(me: Object, s: State)
    local back = me:param(3) - s.left
    if me:param(2) == GREAT then
        all_spin(me, back)
    else
        s.back = back
        me:set_action(SPIN)
    end
end

-- `sub_80CE8D8`: roll out.
local function out(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- sub_80CE8F4
        me.timer = ROUND
        s.left = me:param(3)
        me.phase, me.phase_init = 4, 0
        if me:param(2) ~= 0 and s.hit ~= 0 then
            s.back = me:param(3) - s.left
            me:set_action(SPIN)
            if me:param(2) == GREAT then
                all_spin(me, 0)
            end
        end
    elseif phase == 4 then
        -- sub_80CE932
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            roll(me)
            return
        end
        me:set_coordinates_from_panel()
        local left = s.left - 1
        s.left = left
        if left <= 0 or (me:param(2) ~= 0 and s.hit ~= 0) then
            turn(me, s)
            return
        end
        me.timer = ROUND
    else
        error(string.format("yoyo phase %#x reads past off_80CE8EC", phase))
    end
end

-- `sub_80CE9A4`: spin for the fourth parameter's rounds.
local function spin(me: Object, s: State)
    if s.hit == 2 then
        s.hit = 1
        return
    end
    local phase = me.phase
    if phase == 0 then
        -- sub_80CE9CC
        me.timer = ROUND
        s.left = me:param(4)
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        -- sub_80CE9DC
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        me.collision.region = 1
        local left = s.left - 1
        s.left = left
        if left ~= 0 then
            me.timer = ROUND
            return
        end
        me:set_action(BACK)
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
    else
        error(string.format("yoyo phase %#x reads past off_80CE9C4", phase))
    end
end

-- Home: the destroy state.
local function home(me: Object)
    me:set_lifecycle("destroy")
    me.collision.region = 0
end

-- `sub_80CEA0A`: roll back as far as it went.
local function back(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- sub_80CEA24
        me.timer = ROUND
        s.left = s.back
        me.phase, me.phase_init = 4, 0
        if s.left == 0 then
            home(me)
        end
    elseif phase == 4 then
        -- sub_80CEA42
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            roll(me)
            return
        end
        me:set_coordinates_from_panel()
        local left = s.left - 1
        s.left = left
        if left <= 0 then
            home(me)
            return
        end
        me.timer = ROUND
    else
        error(string.format("yoyo phase %#x reads past off_80CEA1C", phase))
    end
end

-- `sub_80CE87C`: resolve its hits; move unless dimmed.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    if c.hit_flags ~= 0 then
        s.hit = 1
        c.region = 0
    end
    local dimmed = battle.dimmed()
    if not dimmed then
        local a = me.action
        if a == OUT then
            out(me, s)
        elseif a == SPIN then
            spin(me, s)
        elseif a == BACK then
            back(me, s)
        else
            error(string.format("yoyo action %#x reads past off_80CE8CC", a))
        end
    end
    if c.region ~= 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
    if not dimmed then
        me:update_sprite()
    end
    c:present()
end

-- `sub_80CE83C`.
local function init(me: Object)
    me.visible = true
    if not object_setup.sprite_and_collision(me, SPRITE, 0, true, SELF_TYPE, TARGET_TYPE, HIT_MOD) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    local v = me.vel
    me.vel = Vec3.new(me.facing * SPEED, v.y, v.z)
    me.collision:present()
    battle.play_sound(0x120)
    me.lifecycle = "update"
end

-- `sub_80CEA88`: clear the slot that holds it (if it still does), then
-- free it and its collision.
local function gone(me: Object)
    local s = me.state :: State
    local owner = me.related1
    if owner then
        if s.slot == "related" then
            if owner.related1 == me then
                owner.related1 = nil
            end
        else
            local slots = owner.state :: Controller
            local held: Object? = if s.slot == "middle"
                then slots.middle
                elseif s.slot == "below" then slots.below
                else slots.above
            if held == me then
                if s.slot == "middle" then
                    slots.middle = nil
                elseif s.slot == "below" then
                    slots.below = nil
                else
                    slots.above = nil
                end
            end
        end
    end
    me.collision:free()
    me:free()
end

-- `sub_80CE81C`.
function yoyo.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        gone(me)
    end
end

return yoyo
