--!strict
-- TomahawkMan's thrown tomahawk (EXE5's attack object #0xB4, 0x080DCB98; EXE5's own), which his charged attack
-- throws (./charge). It leaves his place 16 pixels up, spinning, four pixels a tick along his row, and at each
-- panel's center it crosses it looks where it is. Thrown along the middle row, it comes back from the first panel
-- with an enemy's body on it, else from the last column. Thrown along an edge row, it turns at the first column with
-- an enemy's body anywhere in it (else at the last), crosses to the other edge row two and a half pixels a tick, and
-- comes back along that row. It hits what it passes once a panel (a hit takes its region off until it is over
-- another panel) and goes once it is off the field. It waits while dimmed.

local collision = require("../../rules/collision")
local marker = require("../../lib/navi-chips/marker")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local tomahawk = {}

export type State = {
    -- The row it was thrown along (Param1).
    row: number,
    -- Its turns so far (Param2): none, then one (it comes back along the middle row, or crosses the column), then
    -- from an edge row a second (back along the other edge row).
    turns: number,
    -- It moves along the column (Param3): the panel's center is tested by Y.
    across: boolean,
}

local SPRITE = asset.sprite("thrown-tomahawk")
local SOUND = asset.sound("tomahawk-throw")
local HEIGHT = 16 * 0x1_0000
-- Its speed along a row and along the column (16.16).
local SPEED, CROSSING_SPEED = 0x4_0000, 0x2_8000
local HIT_MOD = 3
-- An enemy's body on a panel, by its side (0x080DCD74, 0x080DCE00).
local ENEMY_BODY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }

-- 0x0800CB28: `center` lies between where it was and where it is, either end counting.
local function crossed(now: number, was: number, center: number): boolean
    return center >= math.min(now, was) and center <= math.max(now, was)
end

-- The last column the way it flies.
local function last_column(me: Object): number
    return if me.vel.x > 0 then 6 else 1
end

-- 0x080DCD38, thrown along the middle row: at the last column, or on a panel with an enemy's body, it comes back.
local function turn_back(me: Object, s: State)
    if s.turns ~= 0 then
        return
    end
    if me.panel_x ~= last_column(me) and not bit32.btest(field.flags(me.panel_x, me.panel_y), ENEMY_BODY[me.alliance]) then
        return
    end
    local v = me.vel
    me.vel = Vec3.new(-v.x, v.y, v.z)
    s.turns = 1
end

-- 0x080DCD7C, thrown along an edge row: at the last column, or one with an enemy's body in it, it crosses the
-- column; at the other edge row it comes back.
local function turn_around(me: Object, s: State)
    local v = me.vel
    if s.turns == 0 then
        if me.panel_x ~= last_column(me) and #panels.in_column(me.panel_x, ENEMY_BODY[me.alliance], 0) == 0 then
            return
        end
        me.vel = Vec3.new(0, if me.panel_y == 1 then CROSSING_SPEED else -CROSSING_SPEED, v.z)
        s.turns = 1
        s.across = true
    elseif s.turns == 1 then
        if me.panel_y ~= (if s.row == 3 then 1 else 3) then
            return
        end
        me.vel = Vec3.new(-me.facing * SPEED, 0, v.z)
        s.turns = 2
        s.across = false
    end
end

-- 0x080DCE08: in the operation battle, a marker each tick over the panel it flies to (the next one along a row,
-- within the field; the middle row's while it crosses the column).
local function mark(me: Object)
    local v = me.vel
    local x = me.panel_x + (if v.x == 0 then 0 elseif v.x > 0 then 1 else -1)
    if x < 1 or x > 6 then
        x = me.panel_x
    end
    if v.y ~= 0 then
        marker.spawn(me, x, 2, 1, 4, true)
    else
        marker.spawn(me, x, me.panel_y, 1, 1, true)
    end
end

-- 0x080DCC78: a tick of its flight.
local function fly(me: Object, s: State)
    local c = me.collision
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local cx, cy = field.center(me.panel_x, me.panel_y)
    local center = if s.across then cy else cx
    local p, v = me.pos, me.vel
    local x, y = int.i32(p.x + v.x), int.i32(p.y + v.y)
    me.pos = Vec3.new(x, y, p.z)
    local was, now = if s.across then p.y else p.x, if s.across then y else x
    -- Past its panel's center (not while it starts on it): back onto the center, and the turn's test.
    if was ~= center and crossed(now, was, center) then
        me:set_coordinates_from_panel()
        if s.row == 2 then
            turn_back(me, s)
        else
            turn_around(me, s)
        end
    end
    local px, py = me.panel_x, me.panel_y
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= px or me.panel_y ~= py then
        c:set_region(regions.single)
    end
    mark(me)
    if not field.valid(me.panel_x, me.panel_y) then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- 0x080DCC3C.
local function tick(me: Object, s: State)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a ~= 0 then
            error(string.format("the thrown tomahawk's action %#x reads past its table (0x080DCC74)", a))
        end
        fly(me, s)
    end
    c:present()
end

-- 0x080DCBBC: over the panel it was thrown from, 16 pixels up, flying ahead; a slash against navis with no spark.
local function init(me: Object, s: State)
    me:set_panel_from_coordinates()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, int.i32(bit32.band(p.z, 0xFFFF) + HEIGHT))
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    s.row = me.panel_y
    s.turns = 0
    s.across = false
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.slash, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

tomahawk.kind = define.kind {
    id = "tomahawkman/tomahawk",
    pool = "attack",
    state = { row = "u8", turns = "u8", across = "bool" },
    -- 0x080DCB98
    update = function(me: Object)
        local s = me.state :: State
        local l = me.lifecycle
        if l == "init" then
            init(me, s)
        elseif l == "update" then
            tick(me, s)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080DCD16: a tomahawk at `owner`'s place, of his side and facing, with `element` and the damage word `damage`.
-- (The game spawns it at the registers' values and then stores X and Y; its init sets Z's high half alone, so the
-- low half stays what r3 held: the thrower's AI data's address, which his update's 0x080136E0 leaves there. Nothing
-- reads a fraction of a pixel of height: here it is 0, compat's `scratch_z_fraction`.)
function tomahawk.throw(owner: Object, element: number, damage: number): Object?
    local p = owner.pos
    local o = battle.spawn(tomahawk.kind, Vec3.new(p.x, p.y, 0))
    if o then
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

return tomahawk
