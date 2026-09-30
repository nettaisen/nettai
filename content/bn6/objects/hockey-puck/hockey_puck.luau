--!strict
-- A hockey puck (attack object #0x2E, `sub_80C9824`): AirHocky's and
-- PitHocky's. It slides diagonally from panel center to panel center,
-- one step per kind's ticks, bouncing off the field's top and bottom rows
-- and side columns (sound 0xB6). Once it has been over the other side's
-- panels it bounces off its own side's too. Its collision moves to the
-- next panel halfway through each step; a hit takes its region away until
-- it reaches another panel. It lasts its kind's number of steps and bursts
-- (effect 0x14) off solid ground or when the battle ends. PitHocky's
-- (kind 3) leaves an afterimage trail.
--
-- Parameters: 1 the kind (`byte_80C9818`), 2 down (1) or up (0); 3 and 4
-- are its own (steps taken, and whether it has been across).

export type State = {
    -- The panel the step started from (ExtraVars+4, +8), and the center of
    -- the panel it goes to, 16.16 (ExtraVars+0xC, +0x10).
    from_x: number,
    from_y: number,
    to_x: number,
    to_y: number,
}

local hockey_puck = {
    state = { from_x = "i32", from_y = "i32", to_x = "i32", to_y = "i32" },
}

local SPRITE = "10-06"
-- By kind (`byte_80C9818`): the steps it lasts (0: no limit), how it
-- bounces (1: off its own side too, once across), and the ticks per step.
local KINDS = {
    [0] = { steps = 6, own_side = 1, ticks = 6 },
    { steps = 0x0A, own_side = 1, ticks = 6 },
    { steps = 0x0A, own_side = 1, ticks = 6 },
    { steps = 0x12, own_side = 1, ticks = 6 },
}
local PIT = 3
local BOUNCE = 0xB6
-- PitHocky's trail: every fourth tick, an afterimage of the puck for 8
-- ticks, tinted.
local TRAIL_TINT, TRAIL_TICKS = 0x8318, 8
-- The parameters it keeps its own counts in.
local STEPS, ACROSS = 3, 4

-- `sub_80C9BA4`: a puck for `owner` on (x, y) at height `z` (the
-- spawner's registers: the panel Y and the element are its X and Y until
-- its init places it), with the damage word `damage` and the chip's
-- `params`.
function hockey_puck.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    params: { number }
): Object?
    local o = battle.spawn_kind("hockey-puck", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
    end
    return o
end

type Kind = { steps: number, own_side: number, ticks: number }

local function kind_of(me: Object): Kind
    local k = KINDS[me:param(1)]
    if k == nil then
        error(string.format("hockey puck kind %d reads past byte_80C9818", me:param(1)))
    end
    return k
end

-- The panel's owner is the other side (`object_getPanelDataOffset`).
local function other_side(me: Object, x: number, y: number): boolean?
    local p = field.panel(x, y)
    if p == nil then
        return nil
    end
    return p.alliance ~= me.alliance
end

-- `sub_80C9A58`: the next step from (x, y) by (dx, dy), bouncing off the
-- top and bottom rows and the far column. Returns the target's center and
-- the steps.
local function bounce_simple(me: Object, x: number, y: number, dx: number, dy: number): (number, number, number, number)
    if y + dy < 1 or y + dy > 3 then
        dy = -dy
        battle.play_sound(BOUNCE)
    end
    local nx = x + dx
    if me.alliance == 0 then
        if nx > 6 then
            dx = -dx
            battle.play_sound(BOUNCE)
        end
    elseif nx < 1 then
        dx = -dx
        battle.play_sound(BOUNCE)
    end
    local cx, cy = field.center(x + dx, y + dy)
    return cx, cy, dx, dy
end

-- `sub_80C9AAA`: the same off every side of the field; once across (the
-- fourth parameter), a step onto its own side's panels turns back, and
-- if it can't, the puck ends (the steps come back 0).
local function bounce_across(me: Object, x: number, y: number, dx: number, dy: number): (number, number, number, number)
    if y + dy < 1 or y + dy > 3 then
        dy = -dy
        battle.play_sound(BOUNCE)
    end
    if x + dx < 1 or x + dx > 6 then
        dx = -dx
        battle.play_sound(BOUNCE)
    end
    if me:param(ACROSS) ~= 0 and other_side(me, x + dx, y + dy) ~= true then
        dx = -dx
        if other_side(me, x + dx, y + dy) ~= true then
            return 0, 0, 0, dy
        end
        battle.play_sound(BOUNCE)
    end
    local cx, cy = field.center(x + dx, y + dy)
    return cx, cy, dx, dy
end

-- Plan the next step from the panel it's on. False when it can't go on.
local function plan(me: Object, s: State, k: Kind): boolean
    local tx, ty, dx, dy
    if k.own_side == 0 then
        tx, ty, dx, dy = bounce_simple(me, s.from_x, s.from_y, me.vel.x, me.vel.y)
    else
        tx, ty, dx, dy = bounce_across(me, s.from_x, s.from_y, me.vel.x, me.vel.y)
        if dx == 0 then
            return false
        end
    end
    s.to_x, s.to_y = tx, ty
    me.vel = Vec3.new(dx, dy, me.vel.z)
    me:update_collision_panels()
    me.timer = 0
    return true
end

-- `sub_80C9B36`: `timer`/`ticks` of the way from `from`'s center to the
-- target (BIOS Div, truncating).
local function between(s: State, timer: number, ticks: number): (number, number)
    local cx, cy = field.center(s.from_x, s.from_y)
    if ticks == 0 then
        error("hockey puck: its step divides by zero (sub_80C9B36)")
    end
    local x = cx + int.tdiv(int.mul32(s.to_x - cx, timer), ticks)
    local y = cy + int.tdiv(int.mul32(s.to_y - cy, timer), ticks)
    return int.i32(x), int.i32(y)
end

-- `sub_80C9B64`: PitHocky's trail.
local function trail(me: Object)
    if me:param(1) ~= PIT then
        return
    end
    local t = int.u16(me.timer2 + 1)
    me.timer2 = t
    if bit32.band(t, 3) ~= 3 then
        return
    end
    battle.afterimage(me, me.pos, {
        sprite = SPRITE,
        flip = bit32.bxor(me.alliance, me.flip),
        lifetime = TRAIL_TICKS,
        color_shader = TRAIL_TINT,
    })
end

-- Burst: the dust, and the destroy state.
local function burst(me: Object)
    battle.effect(me.pos, 0x14)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- The panel under a position (`sub_800E258`).
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `sub_80C9964`: step along, re-planning at each panel.
local function slide(me: Object)
    trail(me)
    local s = me.state :: State
    local k = kind_of(me)
    if me.phase_init == 0 then
        if not plan(me, s, k) then
            return burst(me)
        end
        me.phase_init = 4
    end
    local ticks = k.ticks
    local t = me.timer + 1
    me.timer = t
    if t == bit32.rshift(ticks, 1) then
        local c = me.collision
        c.panel_x, c.panel_y = panel_at(s.to_x, s.to_y)
    end
    if t == ticks then
        if k.steps ~= 0 then
            local taken = me:param(STEPS) + 1
            me:set_param(STEPS, taken)
            if taken >= k.steps then
                return burst(me)
            end
        end
        s.from_x, s.from_y = me.panel_x, me.panel_y
        if not plan(me, s, k) then
            return burst(me)
        end
        -- (The game divides the next position by the kind's bounce mode
        -- here: the timer is 0, so it lands on the panel it's on.)
        ticks = k.own_side
    end
    local x, y = between(s, me.timer, ticks)
    local p = me.pos
    me.pos = Vec3.new(x, y, p.z)
    me:set_panel_from_coordinates()
    local c = me.collision
    if not field.solid(c.panel_x, c.panel_y) or not field.valid(c.panel_x, c.panel_y) then
        burst(me)
    end
end

-- `sub_80C98DC`: resolve its hits; slide; note when it has been across;
-- its region comes back on a new panel.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        return burst(me)
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    local was_x, was_y = c.panel_x, c.panel_y
    local a = me.action
    if a ~= 0 then
        error(string.format("hockey puck action %#x reads past off_80C9960", a))
    end
    slide(me)
    if me:param(ACROSS) == 0 and other_side(me, c.panel_x, c.panel_y) then
        me:set_param(ACROSS, 1)
    end
    if me.lifecycle ~= "destroy" and (c.panel_x ~= was_x or c.panel_y ~= was_y) then
        c.region = 1
    end
    c:present()
    field.highlight(c.panel_x, c.panel_y)
end

-- `sub_80C9848`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0xFF
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local s = me.state :: State
    s.from_x, s.from_y = me.panel_x, me.panel_y
    kind_of(me)
    -- Its step, in panels.
    me.vel = Vec3.new(me.facing, me:param(2) * 2 - 1, me.vel.z)
    me:set_param(ACROSS, 0)
    me:set_param(STEPS, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(6, 5, 1)
    me.collision.hit_effect = 0x0A
    me.collision:present()
    battle.play_sound(0xB4)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C9824`.
function hockey_puck.update(me: Object)
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
    me:update_sprite()
end

return hockey_puck
