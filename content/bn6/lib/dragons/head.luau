--!strict
-- A dragon's head (attack object #0xC9, `sub_80DE404`): HeatDrgn's,
-- ElecDrgn's, AquaDrgn's or WoodDrgn's (its variant). It rises from the
-- row above the field, dives down its column hitting each panel it passes,
-- swims a panel forward, and climbs back out; the elec, aqua and wood
-- dragons leave the panels behind them cracked, frozen or grassy. Four body
-- segments follow it (./body). See ./dragon and docs/engine/chips.md §3.8.

local body = require("./body")
local collision = require("../../rules/collision")
local dragon = require("./dragon")
local regions = require("../regions")

local head = {}

export type State = {
    -- The panel it was last on (EV+0, EV+1).
    last_x: number,
    last_y: number,
    -- Ticks into the swim, or left in the climb out (EV+2).
    steps: number,
    -- Down or up its column (EV+4).
    leg: number,
    -- Ticks a panel takes (EV+5), and the dip's angle step and angle
    -- (EV+6, EV+7).
    ticks_per_panel: number,
    angle_step: number,
    angle: number,
    -- Its height where the dip starts (EV+8).
    base_y: number,
    -- Its dragon (the original's first parameter, the chip's subtype).
    variant: DragonVariant,
}

-- Collision: what it is and hits.
local HIT = { is = collision.attack, hits = collision.hits_navis }
-- What ends its hits (`dword_80DE540`).
local BLOCKED = 0xFF80_0000
-- Its swimming animations: the first, and which way they run, by leg
-- (`dword_80DE608`, `byte_80DE610`).
local SWIM_ANIM = { [0] = 2, 6 }
local SWIM_ANIM_DIR = { [0] = 1, -1 }
local HEAD_ANIM = 2
local ROAR = asset.sound("form-change")
-- Its actions: dive or climb, swim, leave.
local COLUMN, SWIM, LEAVE = 0, 4, 8

-- `sub_80DE768`: the variant's panel on (x, y), if that is a solid field
-- panel.
local function mark(me: Object, x: number, y: number)
    if x <= 0 or x >= 7 or y <= 0 or y >= 4 or not field.solid(x, y) then
        return
    end
    local panel = (me.state :: State).variant.panel
    if panel then
        field.set_type(x, y, panel)
    end
end

-- `sub_80DE730`: on a new panel, mark the one it left and hit again.
local function crossed(me: Object)
    local s = me.state :: State
    local x, y = me.panel_x, me.panel_y
    if x == s.last_x and y == s.last_y then
        return
    end
    mark(me, s.last_x, s.last_y)
    s.last_x, s.last_y = x, y
    me.collision:set_region(regions.single)
end

-- `sub_80DE544`: down (or up) its column; at the bottom row, swim; at the
-- row above the field, leave.
local function column(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    crossed(me)
    local s = me.state :: State
    if me.panel_y ~= bit32.bxor(s.leg, 1) * 3 then
        return
    end
    if s.leg == dragon.UP then
        s.steps = dragon.exit_ticks(me, s.leg)
        me.action = LEAVE
        me.phase = 0
    else
        s.angle = 0
        s.base_y = me.pos.y
        me.action = SWIM
    end
end

-- `sub_810FA4C`: which fifth of the panel it is through (0 to 4).
local function fifth(steps: number, per_panel: number): number
    local n = int.tdiv(per_panel, 5)
    if n == 0 then
        error("sub_810FA4C divides by zero")
    end
    return math.min(int.tdiv(steps, n), 4)
end

-- `sub_80DE594`: a panel forward in a dip, turning its head; then turn
-- up.
local function swim(me: Object)
    local s = me.state :: State
    local steps = s.steps + 1
    s.steps = steps
    local p, v = me.pos, me.vel
    if steps == s.ticks_per_panel then
        s.leg = bit32.bxor(s.leg, 1)
        local cx = field.center(me.panel_x, me.panel_y)
        me.vel = Vec3.new(v.x, -v.y, v.z)
        me.pos = Vec3.new(cx, p.y, p.z)
        s.steps = 0
        me.action = COLUMN
    else
        me.anim = fifth(steps, s.ticks_per_panel) * SWIM_ANIM_DIR[s.leg] + SWIM_ANIM[s.leg]
        me.pos = Vec3.new(p.x + v.x, p.y, p.z)
        local angle = s.angle + s.angle_step
        s.angle = angle
        dragon.dip(me, angle, s.base_y, s.leg)
        me:set_panel_from_coordinates()
        crossed(me)
    end
    me:update_collision_panels()
end

-- `sub_80DE618`: climb out of sight, then splash and go.
local function leave(me: Object)
    local s = me.state :: State
    if s.steps ~= 0 then
        s.steps -= 1
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x, p.y + v.y, p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        crossed(me)
        return
    end
    if me.phase == 0 then
        me.phase = 1
        dragon.splash(me, s.leg)
    end
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80DE4EC`: resolve the last hits (a blocked one stops its hits
-- until the next panel), move (not while dimmed), register again and
-- highlight where it hits.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if bit32.btest(c.hit_flags, BLOCKED) then
        c:set_region(nil)
    end
    if not battle.dimmed() then
        local a = me.action
        if a == COLUMN then
            column(me)
        elseif a == SWIM then
            swim(me)
        elseif a == LEAVE then
            leave(me)
        else
            error(string.format("dragon head action %#x reads past its table", a))
        end
    end
    c:present()
    me:highlight_collision_panels()
end

-- `sub_80DE430`: in the row above the field, with its body behind; a
-- splash and a roar.
local function init(me: Object): boolean
    local s = me.state :: State
    local v = s.variant
    dragon.place(me, v, HEAD_ANIM)
    me.visible = true
    if not me:create_collision() then
        me:free()
        return false
    end
    me:setup_collision(HIT.is, HIT.hits, v.hit_mod)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(v.spark)
    c:present()
    -- sub_80DE67E
    s.last_x, s.last_y = me.panel_x, me.panel_y
    local per_panel, step = dragon.start(me, v)
    s.ticks_per_panel = per_panel
    if per_panel ~= 0 then
        s.angle_step = step
    end
    s.leg = dragon.DOWN
    s.steps = 0
    -- sub_80DE7C8
    for i = 0, 3 do
        body.spawn(me, v, i, v.delays[i + 1])
    end
    dragon.splash(me, 1)
    battle.play_sound(ROAR)
    me:set_lifecycle("update")
    tick(me)
    return true
end

head.kind = define.kind {
    id = "dragon-head",
    pool = "attack",
    state = {
        last_x = "u8",
        last_y = "u8",
        steps = "u8",
        leg = "u8",
        ticks_per_panel = "u8",
        angle_step = "u8",
        angle = "u8",
        base_y = "i32",
        variant = "record:dragon-variant",
    },
    -- `sub_80DE404`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if not init(me) then
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

-- `sub_80DE660`: dragon `v` in column x for `owner`, with the damage word
-- `damage`. Its position is the spawner's registers (the row, the element
-- and z) until its init places it.
function head.spawn(owner: Object, x: number, y: number, element: number, v: DragonVariant, damage: number): Object?
    local o = battle.spawn(head.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.variant = v
    end
    return o
end

return head
