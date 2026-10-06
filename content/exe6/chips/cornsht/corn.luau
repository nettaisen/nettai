--!strict
-- CornSht's corn (the original's attack object #0x10, `sub_80C6580`): an
-- unseen probe that steps a panel a tick from where it appears; where it
-- finds something it bursts twice on that panel (13 ticks each), turns it
-- to grass, and on the twelfth tick of the first burst spreads a corn to
-- each enemy body in the column beyond, which bursts in turn and spreads
-- on. See docs/engine/shot-chips.md §9.1.

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local corn = {}

export type State = {
    -- Its generation (Param1): 0 the probe; a spread corn's is its
    -- parent's + 1; 0xFF bursts without spreading (CornFsta's).
    generation: number,
    -- Bursts left (Param2).
    bursts: number,
    -- Whether its first burst has passed (and grassed its panel), and a
    -- count of its burst ticks (ExtraVars+0xC, +8).
    grassed: boolean,
    ticks: number,
    -- The panels its spread goes to, x | y << 4 (ExtraVars+0x64..), and
    -- how many (ExtraVars).
    spread: { number },
    count: number,
}

-- A generation that bursts and doesn't spread.
corn.NO_SPREAD = 0xFF

local BURSTS = 2
-- Each burst's ticks (less one), and the tick of the first the spread
-- goes on (Timer2: it counts down through both bursts, so it reaches 0
-- once).
local BURST_TICKS, SPREAD_TICKS = 0xC, 0xC
local HIT_MOD = 1
local EFFECT = new.effect { sprite = asset.sprite("energy-burst"), anim = 0, palette = 1 }
local SOUND = asset.sound("hit-bomb-1")
-- The other side's body on a panel (`off_80C6764`, by side).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
-- A solid panel (it turns to grass under the first burst).
local SOLID = 0x10

-- `sub_80C673A`: the panels of the column around the one ahead with an
-- enemy's body on them, in the region's order.
local function find_spread(me: Object, s: State)
    local x, y = me.panel_x + me.facing, me.panel_y
    local dir = panels.side_direction(bit32.bxor(me.alliance, me.flip))
    local n = 0
    for _, p in regions.wide.panels or {} do
        local px, py = x + p[1] * dir, y + p[2]
        if field.check(px, py, ENEMY_BODY[me.alliance], 0) then
            n += 1
            s.spread[n] = panels.pack(px, py)
        end
    end
    s.count = n
end

-- `sub_80C67A4`: whether some corn already stands on (x, y) (any in the
-- update list, this one too). The game's walk answers with x when none
-- does, so column 0 always counts as taken.
local function taken(x: number, y: number): boolean
    if x == 0 then
        return true
    end
    local list: { Object } = battle.objects_of(corn.kind :: Kind)
    for _, o in list do
        if o.panel_x == x and o.panel_y == y then
            return true
        end
    end
    return false
end

-- `sub_80C6774`: a corn of the next generation on each panel found, last
-- first, unless one stands there.
local function spread(me: Object, s: State)
    local damage = int.u32(me.damage + me.stamina * 0x10000)
    for i = s.count, 1, -1 do
        local p = s.spread[i]
        local x, y = panels.x(p), panels.y(p)
        if not taken(x, y) then
            corn.spawn(me, x, y, 0, me.element, damage, bit32.band(s.generation + 1, 0xFF))
        end
    end
end

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80C6650`: a panel forward; off the field it ends.
local function step(me: Object)
    me.panel_x = me.panel_x + me.facing
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
    end
end

-- `sub_80C6698`: a burst's start: the first grasses a solid panel under
-- it; an attack of its own element now (`sub_80C6726`), over its panel,
-- with the effect and the sound.
local function burst(me: Object, s: State)
    if not s.grassed then
        s.grassed = true
        if field.check(me.panel_x, me.panel_y, SOLID, 0) then
            field.set_type(me.panel_x, me.panel_y, "grass")
        end
    end
    me:reset_collision_types(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision.element = bit32.band(me.element, 0xF)
    me.collision.secondary_element = bit32.band(me.element, 0xF0)
    me.collision:set_region(regions.single)
    battle.effect(me.pos, EFFECT)
    battle.play_sound(SOUND)
    me.timer = BURST_TICKS
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C66E4`: the sound every 8 ticks; the spread when Timer2 reaches
-- 0; at the burst's end the next, or done.
local function bursting(me: Object, s: State)
    local t = s.ticks
    s.ticks = int.u32(t + 1)
    if bit32.band(t, 7) == 0 then
        battle.play_sound(SOUND)
    end
    local wait = me.timer2 - 1
    me.timer2 = bit32.band(wait, 0xFFFF)
    if wait == 0 then
        spread(me, s)
    end
    local left = me.timer - 1
    me.timer = bit32.band(left, 0xFFFF)
    if left >= 0 then
        return
    end
    s.bursts -= 1
    if s.bursts ~= 0 then
        me.phase, me.phase_init = 0, 0
    else
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C65EE`: the hits against it resolve and spark; a probe that
-- found something bursts there (a burst that landed leaves its panel).
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        if me.action == 4 then
            c:set_region(nil)
        else
            find_spread(me, s)
            me.action = 4
            me.phase, me.phase_init = 0, 0
        end
    end
    if battle.over() then
        finish(me)
        return
    end
    if me.action == 0 then
        step(me)
    elseif me.phase == 0 then
        burst(me, s)
    else
        bursting(me, s)
    end
    c:present()
end

-- `sub_80C65A0`: on its panel, unseen, a probe; a spread corn bursts
-- where it is, and finds where it spreads to now.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.probe, collision.hits_navis, 0)
    me.collision.element = 0
    me.collision:present()
    local s = me.state :: State
    s.bursts = BURSTS
    me.timer2 = SPREAD_TICKS
    me:set_lifecycle("update")
    if s.generation ~= 0 then
        me.action = 4
        if s.generation ~= corn.NO_SPREAD then
            find_spread(me, s)
        end
    end
end

corn.kind = new.kind {
    id = "cornsht/corn",
    pool = "attack",
    state = { generation = "u8", bursts = "u8", grassed = "bool", ticks = "u32", spread = "u8[3]", count = "u8" },
    -- `sub_80C6580`.
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
}

-- `sub_80C67D8`: a corn of `generation` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element` and the damage word; nil if the
-- attack pool is full. It takes the owner's side and flip, runs later
-- this tick, right after its spawner, and keeps its Z.
function corn.spawn(owner: Object, x: number, y: number, z: number, element: number, damage: number, generation: number): Object?
    local o = battle.spawn(corn.kind, Vec3.new(0, 0, z))
    if o then
        (o.state :: State).generation = generation
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return corn
