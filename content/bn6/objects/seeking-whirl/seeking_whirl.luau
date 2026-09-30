--!strict
-- A seeking whirlwind (attack object #0xD4, `sub_80DFFB8`): AirSpin's
-- variant 1 (no chip in the table uses it). It swoops down from behind and
-- above onto its panel in 19 ticks, then moves a panel every 6 ticks
-- toward the enemy: straight ahead, or diagonally when a visible enemy
-- navi stands on a panel ahead one row up or down (and on diagonally from
-- then on). On a panel with a visible enemy navi it stays, hitting up to 3
-- times there. It blows away off solid ground or off the field. Its region
-- comes back 8 ticks after each hit.

export type State = {
    -- The coordinates of the panel it moves to (ExtraVars+0, +4), and the
    -- panel it moves from (ExtraVars+8, +0xC).
    target_x: number,
    target_y: number,
    from_x: number,
    from_y: number,
    -- Hits on this panel (ExtraVars+0x10), and ticks until its region
    -- comes back (ExtraVars+0x14).
    hits: number,
    regrow: number,
    -- Its step: X direction and row change (ExtraVars+0x18, +0x1C).
    dx: number,
    dy: number,
}

local seeking_whirl = {
    state = {
        target_x = "i32",
        target_y = "i32",
        from_x = "i32",
        from_y = "i32",
        hits = "i32",
        regrow = "i32",
        dx = "i32",
        dy = "i32",
    },
}

local SPRITE = "10-0d"
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 0x04, 0x05, 0x01
local SOUND_COME = 0x140
-- Where it starts: 80 pixels behind its panel, 160 up; how it swoops.
local BEHIND, HEIGHT = 0x50, 0xA0
local SWOOP_X, SWOOP_Z, SWOOP_TICKS = 0x4_0000, -0x8_0000, 0x14
-- Ticks a panel, hits a panel, ticks before its region comes back.
local STEP_TICKS, MAX_HITS, REGROW = 6, 3, 8
-- The enemy's bodies on a panel's flags word, by its side
-- (`byte_80E02EC`).
local ENEMY_BODIES = { [0] = 0x05800000, [1] = 0x0A800000 }

-- The actions.
local SWOOP, MOVE, HOLD, BLOW_AWAY = 0x0, 0x4, 0x8, 0xC

-- `sub_80E025E`: from (x, y), the first panel forward that is the other
-- side's (`object_getFirstPanelInDirectionFiltered` with `byte_80E029C`);
-- a seeking whirlwind for `owner` onto it (`sub_80E0248`), if any. Its
-- position is the spawner's registers (the panel Y, the element, and the
-- filter's forbidden bits) until its init places it.
function seeking_whirl.send(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local dir = owner.facing
    local require, forbid = 0x20, 0
    if owner.alliance ~= 0 then
        require, forbid = 0, 0x20
    end
    while not field.check(x, y, require, forbid) do
        x += dir
        if not field.valid(x, y) then
            return nil
        end
    end
    -- (The search answers with the panel's X, so a find in column 0, off
    -- the field, reads as none.)
    if x == 0 then
        return nil
    end
    local o = battle.spawn_kind("seeking-whirl", Vec3.new(y, element, forbid), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80E02AC`: a visible enemy navi stands on (x, y).
local function enemy_on(me: Object, x: number, y: number): boolean
    if not bit32.btest(field.flags(x, y), ENEMY_BODIES[me.alliance]) then
        return false
    end
    -- sub_800AE54: the other side's first combatant on (x, y).
    for _, a in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        if a.panel_x == x and a.panel_y == y then
            return not (a:status("invisible") or a:status("submerged"))
        end
    end
    return false
end

-- `sub_80E02F4`: the next panel forward, one row up or down instead if an
-- enemy stands there and not straight ahead: its coordinates, the X
-- direction and the row change.
local function next_step(me: Object): (number, number, number, number)
    local dir = me.facing
    local x, y = me.panel_x + dir, me.panel_y
    local dy = 0
    if not enemy_on(me, x, y) then
        if enemy_on(me, x, y - 1) then
            dy, y = -1, y - 1
        elseif enemy_on(me, x, y + 1) then
            dy, y = 1, y + 1
        end
    end
    local cx, cy = field.center(x, y)
    return cx, cy, dir, dy
end

-- Aim at the next panel: a new choice while it moves straight
-- (`sub_80E02F4`), or on along its diagonal (`sub_80E0376`).
local function aim(me: Object, s: State)
    local cx, cy
    if s.dy == 0 then
        local dx, dy
        cx, cy, dx, dy = next_step(me)
        s.dx, s.dy = dx, dy
    else
        cx, cy = field.center(me.panel_x + s.dx, me.panel_y + s.dy)
    end
    s.target_x, s.target_y = cx, cy
    me:update_collision_panels()
end

-- `sub_800E258`: the panel under coordinates (x, y).
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `object_clearCollisionRegion`, then the last action.
local function blow_away(me: Object)
    me.collision.region = 0
    me:set_action(BLOW_AWAY)
end

-- `sub_80E00B0`: swoop down onto its panel; there, hold if an enemy is on
-- it, else move.
local function swoop(me: Object, s: State)
    if me.phase_init == 0 then
        me.vel = Vec3.new(me.facing * SWOOP_X, me.vel.y, SWOOP_Z)
        me.timer = SWOOP_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        me.pos = me.pos + Vec3.new(me.vel.x, 0, me.vel.z)
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.collision.region = 1
    s.from_x, s.from_y = me.panel_x, me.panel_y
    s.dx, s.dy = 0, 0
    me:set_action(if enemy_on(me, me.panel_x, me.panel_y) then HOLD else MOVE)
end

-- `sub_80E0118`: glide to the next panel in 6 ticks (its region moving on
-- halfway), then choose again; stop on a panel with an enemy.
local function move(me: Object, s: State)
    local c = me.collision
    if not field.solid(c.panel_x, c.panel_y) then
        return blow_away(me)
    end
    if me.phase_init == 0 then
        aim(me, s)
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer + 1
    me.timer = t
    if t == STEP_TICKS // 2 then
        c.panel_x, c.panel_y = panel_at(s.target_x, s.target_y)
    end
    if t == STEP_TICKS then
        s.from_x, s.from_y = me.panel_x, me.panel_y
        aim(me, s)
        me.timer = 0
        if enemy_on(me, me.panel_x, me.panel_y) then
            me:set_coordinates_from_panel()
            me:set_action(HOLD)
            return
        end
    end
    -- sub_80E0348: from the panel it left toward the target, timer/6 of
    -- the way (the game's truncating division).
    local fx, fy = field.center(s.from_x, s.from_y)
    local n = me.timer
    local x = fx + int.tdiv(int.mul32(int.i32(s.target_x - fx), n), STEP_TICKS)
    local y = fy + int.tdiv(int.mul32(int.i32(s.target_y - fy), n), STEP_TICKS)
    me.pos = Vec3.new(x, y, me.pos.z)
    me:set_panel_from_coordinates()
    if not field.valid(c.panel_x, c.panel_y) then
        blow_away(me)
    end
end

-- `sub_80E01EE`: stay on the enemy's panel for up to 3 hits.
local function hold(me: Object, s: State)
    local c = me.collision
    if not field.solid(c.panel_x, c.panel_y) then
        return blow_away(me)
    end
    if enemy_on(me, me.panel_x, me.panel_y) and s.hits < MAX_HITS then
        return
    end
    me:set_action(MOVE)
end

-- `sub_80E0226`: blow away in 4 ticks.
local function vanish(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
        me.timer = 5
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: (Object, State) -> () } = {
    [SWOOP] = swoop,
    [MOVE] = move,
    [HOLD] = hold,
    [BLOW_AWAY] = vanish,
}

-- `sub_80E002C`: its hits (each takes its region away for 8 ticks), the
-- action, and a fresh count of hits on each new panel.
local function tick(me: Object)
    local c = me.collision
    local s = me.state :: State
    c:remove()
    if c.hit_flags ~= 0 then
        c.region = 0
        s.hits += 1
        s.regrow = REGROW
    end
    local a = me.action
    if a ~= SWOOP and a ~= BLOW_AWAY and c.region == 0 then
        local left = s.regrow - 1
        s.regrow = left
        if left <= 0 then
            c.region = 1
        end
    end
    local was_x, was_y = c.panel_x, c.panel_y
    local act = ACTIONS[a]
    if act == nil then
        error(string.format("seeking whirl action %#x reads past its table", a))
    end
    act(me, s)
    if c.panel_x ~= was_x or c.panel_y ~= was_y then
        s.hits = 0
        if me.action ~= BLOW_AWAY then
            c.region = 1
        end
    end
    field.highlight(c.panel_x, c.panel_y)
    c:present()
end

-- `sub_80DFFDC`: behind and above its panel, with no region yet.
local function init(me: Object)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x - me.facing * BEHIND * 0x10000), p.y, HEIGHT * 0x10000)
    -- sub_8011504
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        error("seeking whirl: no collision slot (sub_8011504; the game reads through a null pointer)")
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision.region = 0
    me.collision:present()
    battle.play_sound(SOUND_COME)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80DFFB8`.
function seeking_whirl.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite()
end

return seeking_whirl
