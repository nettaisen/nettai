--!strict
-- A hockey puck (the original's attack object #0x2E, `sub_80C9824`):
-- AirHocky's and PitHocky's. It slides diagonally from panel center to
-- panel center, one step per its variant's ticks, bouncing off the field's
-- top and bottom rows and side columns. Once it has been over the other
-- side's panels it bounces off its own side's too. Its collision moves to
-- the next panel halfway through each step; a hit takes its region away
-- until it reaches another panel. It lasts its variant's number of steps
-- and bursts off solid ground or when the battle ends. PitHocky's leaves
-- an afterimage trail.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

local puck = {}

-- A puck's variant (`HockeyPuckVariant`, types.d.luau; a row of
-- `byte_80C9818`).
export type Variant = HockeyPuckVariant

export type State = {
    variant: Variant,
    -- It goes down the field (1) or up (0) (Param2).
    down: boolean,
    -- Steps taken (Param3), and whether it has been across (Param4).
    steps: number,
    across: boolean,
    -- The panel the step started from (ExtraVars+4, +8), and the center of
    -- the panel it goes to, 16.16 (ExtraVars+0xC, +0x10).
    from_x: number,
    from_y: number,
    to_x: number,
    to_y: number,
}

function puck.variant(v: Variant): Variant
    return define.record("hockey-puck-variant", v)
end

-- `byte_80C9818`'s rows: the steps it lasts (0: no limit), how it bounces
-- (1: off its own side too, once across; the game also divides by it, see
-- `slide`), and the ticks per step; the trail is PitHocky's. The first two
-- rows no chip has.
local ROWS: { [number]: Variant } = {
    [0] = puck.variant { steps = 6, own_side = 1, ticks = 6 },
    [1] = puck.variant { steps = 0x0A, own_side = 1, ticks = 6 },
    [2] = puck.variant { steps = 0x0A, own_side = 1, ticks = 6 },
    [3] = puck.variant { steps = 0x12, own_side = 1, ticks = 6, trail = true },
}
-- The chips' pucks.
puck.variants = { airhocky = ROWS[2], pithocky = ROWS[3] }

local SPRITE = asset.sprite("bn6:hockey-puck")
local SOUND = asset.sound("bn6:hockey-puck")
local SOUND_BOUNCE = asset.sound("bn6:bounce")
-- PitHocky's trail: every fourth tick, an afterimage of the puck for 8
-- ticks, tinted.
local TRAIL_TINT, TRAIL_TICKS = 0x8318, 8

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
        battle.play_sound(SOUND_BOUNCE)
    end
    local nx = x + dx
    if me.alliance == 0 then
        if nx > 6 then
            dx = -dx
            battle.play_sound(SOUND_BOUNCE)
        end
    elseif nx < 1 then
        dx = -dx
        battle.play_sound(SOUND_BOUNCE)
    end
    local cx, cy = field.center(x + dx, y + dy)
    return cx, cy, dx, dy
end

-- `sub_80C9AAA`: the same off every side of the field; once across, a step
-- onto its own side's panels turns back, and if it can't, the puck ends
-- (the steps come back 0).
local function bounce_across(me: Object, x: number, y: number, dx: number, dy: number): (number, number, number, number)
    if y + dy < 1 or y + dy > 3 then
        dy = -dy
        battle.play_sound(SOUND_BOUNCE)
    end
    if x + dx < 1 or x + dx > 6 then
        dx = -dx
        battle.play_sound(SOUND_BOUNCE)
    end
    if (me.state :: State).across and other_side(me, x + dx, y + dy) ~= true then
        dx = -dx
        if other_side(me, x + dx, y + dy) ~= true then
            return 0, 0, 0, dy
        end
        battle.play_sound(SOUND_BOUNCE)
    end
    local cx, cy = field.center(x + dx, y + dy)
    return cx, cy, dx, dy
end

-- Plan the next step from the panel it's on. False when it can't go on.
local function plan(me: Object, s: State): boolean
    local tx, ty, dx, dy
    if s.variant.own_side == 0 then
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
local function trail(me: Object, s: State)
    if not s.variant.trail then
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
    battle.effect(me.pos, effects.puff)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- The panel under a position (`sub_800E258`).
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `sub_80C9964`: step along, re-planning at each panel.
local function slide(me: Object)
    local s = me.state :: State
    trail(me, s)
    local v = s.variant
    if me.phase_init == 0 then
        if not plan(me, s) then
            return burst(me)
        end
        me.phase_init = 4
    end
    local ticks = v.ticks
    local t = me.timer + 1
    me.timer = t
    if t == bit32.rshift(ticks, 1) then
        local c = me.collision
        c.panel_x, c.panel_y = panel_at(s.to_x, s.to_y)
    end
    if t == ticks then
        if v.steps ~= 0 then
            local taken = int.u8(s.steps + 1)
            s.steps = taken
            if taken >= v.steps then
                return burst(me)
            end
        end
        s.from_x, s.from_y = me.panel_x, me.panel_y
        if not plan(me, s) then
            return burst(me)
        end
        -- (The game divides the next position by the variant's bounce mode
        -- here: the timer is 0, so it lands on the panel it's on.)
        ticks = v.own_side
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
        c:set_region(nil)
    end
    local was_x, was_y = c.panel_x, c.panel_y
    local a = me.action
    if a ~= 0 then
        error(string.format("hockey puck action %#x reads past off_80C9960", a))
    end
    slide(me)
    local s = me.state :: State
    if not s.across and other_side(me, c.panel_x, c.panel_y) then
        s.across = true
    end
    if me.lifecycle ~= "destroy" and (c.panel_x ~= was_x or c.panel_y ~= was_y) then
        c:set_region(regions.single)
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
    -- Its step, in panels.
    me.vel = Vec3.new(me.facing, if s.down then 1 else -1, me.vel.z)
    s.across = false
    s.steps = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.breaking, collision.hits_navis, 1)
    me.collision:set_hit_effect(sparks.breaking)
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

puck.kind = define.kind {
    id = "bn6:airhocky/puck",
    pool = "attack",
    state = {
        variant = "record:hockey-puck-variant",
        down = "bool",
        steps = "u8",
        across = "bool",
        from_x = "i32",
        from_y = "i32",
        to_x = "i32",
        to_y = "i32",
    },
    -- `sub_80C9824`.
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
        me:update_sprite()
    end,
}

-- `sub_80C9BA4`: a puck of `variant` for `owner` on (x, y) at height `z`
-- (the spawner's registers: the panel Y and the element are its X and Y
-- until its init places it), with the damage word `damage`, going down
-- the field or up.
function puck.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    variant: Variant,
    down: boolean
): Object?
    local o = battle.spawn(puck.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.variant, s.down = variant, down
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
    end
    return o
end

return puck
