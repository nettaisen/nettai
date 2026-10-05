--!strict
-- RskyHny's bee (attack object #0x74, `sub_80D30D0`). It flies panel by
-- panel toward the nearest column holding an enemy, turning at most twice,
-- and hits what it flies into. On an enemy's body it stays on them and
-- stings every 5 ticks, five times in all; on anything else that hits back
-- it goes. What it is (`HoneyBee`, types.d.luau: the chip's level, the
-- original's first parameter, by `byte_80D3164`) is its palette and
-- speeds. See docs/engine/chips.md §3.7.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local bee = {}

export type State = {
    -- Its speed across and along the field (EV+0, EV+4), 16.16.
    speed: number,
    turn_speed: number,
    -- The enemy it stings (EV+8).
    target: Object?,
    -- Its palette, and the turns it has taken (the original keeps them in
    -- its first and third parameters).
    palette: number,
    turns: number,
}

local SPRITE = asset.sprite("honey-bee")
-- What it is and hits (`dword_80D34A8`).
local HIT = { is = collision.attack, hits = collision.hits_navis }
local HIT_MOD = 0x01
-- A panel's width, 16.16.
local PANEL_WIDTH = 0x28_0000
-- It gives up turning after this many turns (`sub_80D3496`).
local TURNS = 2
-- Collision results: something that ends it (attacks and the like), and a
-- body it stays on (`dword_80D34B0`, `byte_80D34B4`).
local ENDS_IT, BODIES = 0xF380_0000, 0x0C00_0000
-- A body of the other side on a panel, by its own side (`off_80D33F4`).
local ENEMY_BODY = { [0] = 0x0400_0000, 0x0800_0000 }
-- The other side's navi on a panel, by its own side (`byte_80E74C4`).
local ENEMY_NAVI = { [0] = 0x0020_0000, 0x0040_0000 }
-- Stings, and the ticks between them.
local STINGS, STING_TICKS = 5, 5
-- Its puff, 16 pixels up, when it can't start.
local PUFF_Z = 0x10_0000
-- Its actions.
local FLY, STING, FADE = 0, 4, 8

-- `sub_80E7486`: the other side's combatant whose collision is on (x,
-- y), if the panel shows the other side's navi.
local function enemy_on(x: number, y: number, side: number): Object?
    if not bit32.btest(field.flags(x, y), ENEMY_NAVI[side]) then
        return nil
    end
    for s = 0, 1 do
        for _, o in battle.alive_actors(s) do
            local c = o.collision
            if c.panel_x == x and c.panel_y == y then
                return o
            end
        end
    end
    return nil
end

-- `sub_80D3374`: from column x toward `step`, the first column with an
-- enemy body in it; of its panels, the one in the bee's row, else the
-- nearest row (the upper one on a tie). nil if none.
local function nearest_in_columns(me: Object, x: number, step: number): (number?, number)
    local body = ENEMY_BODY[me.alliance]
    while true do
        local rows = {}
        for y = 1, 3 do
            if field.check(x, y, body, 0) then
                table.insert(rows, y)
            end
        end
        if #rows > 0 then
            local my = me.panel_y
            local best = rows[1]
            for i = 2, #rows do
                local y = rows[i]
                if y == my then
                    best = y
                else
                    local d, bd = math.abs(y - my), math.abs(best - my)
                    if d < bd or (d == bd and y < best) then
                        best = y
                    end
                end
            end
            return x, best
        end
        x += step
        if x < 1 or x > 6 then
            return nil, 0
        end
    end
end

-- `sub_80D3342`: where it heads: the nearest enemy ahead of its user's
-- column, else behind it, else off the far edge in the middle row.
local function destination(me: Object): (number, number)
    local dir = me.facing
    local user = me.related1 :: Object
    local x, y = nearest_in_columns(me, user.panel_x + dir, dir)
    if x then
        return x, y
    end
    x, y = nearest_in_columns(me, user.panel_x, -dir)
    if x then
        return x, y
    end
    return bit32.bxor(me.alliance, 1) * 7, 2
end

-- `sub_80D3496`: one more turn taken; whether turns had run out.
local function out_of_turns(me: Object): boolean
    local s = me.state :: State
    local turns = s.turns
    s.turns = turns + 1
    return turns >= TURNS
end

-- `sub_80D3326`: at a panel's center, turn toward the destination once
-- it is level with it (flying up or down: turn across toward it, or back
-- if it is in this column; flying across: turn up or down toward it, or
-- back if it is in this row), then face the way it flies.
local function steer(me: Object)
    local tx, ty = destination(me)
    local s = me.state :: State
    local v = me.vel
    if v.x == 0 then
        -- sub_80D3404
        local ahead, here = ty, me.panel_y
        if v.y < 0 then
            ahead, here = -ahead, -here
        end
        if ahead <= here and not out_of_turns(me) then
            local x = me.panel_x
            if tx == x then
                me.vel = Vec3.new(v.x, -v.y, v.z)
            else
                me.vel = Vec3.new(s.speed * (if tx > x then 1 else -1), 0, v.z)
            end
        end
    else
        -- sub_80D343C
        local ahead, here = tx, me.panel_x
        if v.x < 0 then
            ahead, here = -ahead, -here
        end
        if ahead <= here and not out_of_turns(me) then
            local y = me.panel_y
            if ty == y then
                me.vel = Vec3.new(-v.x, v.y, v.z)
            else
                me.vel = Vec3.new(0, s.turn_speed * (if ty > y then 1 else -1), v.z)
            end
        end
    end
    -- sub_80D3474: face across the way it flies (the enemy's way is
    -- unflipped).
    local vx = me.vel.x
    if vx ~= 0 then
        local enemy_way = 1 - 2 * me.alliance
        me.flip = if (enemy_way < 0) == (vx < 0) then 0 else 1
        me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    end
end

-- The end: no more hits, and gone at its next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D31EC`: fly; at each panel's center, steer.
local function fly(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
        me.action = 2
        return
    end
    if me.phase == 0 then
        me.phase = 1
        steer(me)
        me.timer = me.timer2
        me:set_coordinates_from_panel()
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me.phase = 0
    end
end

-- `sub_80D3278`: stay on the target; sting every 5 ticks, then go.
local function stay(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.timer = STING_TICKS
    end
    local target = (me.state :: State).target
    if target then
        me.panel_x, me.panel_y = target.panel_x, target.panel_y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        me.collision:set_region(regions.single)
        me.phase_init = 0
    else
        finish(me)
    end
end

-- `ho_80D3240`: find the enemy it flew into (`sub_80D325C`), then stay.
local function sting(me: Object)
    local phase = me.phase
    if phase == 0 then
        local s = me.state :: State
        s.target = enemy_on(me.panel_x, me.panel_y, me.alliance)
        me.timer2 = STINGS
        me.phase = 4
        me.phase_init = 0
        stay(me)
    elseif phase == 4 then
        stay(me)
    else
        error(string.format("honey bee sting phase %#x reads past its table", phase))
    end
end

-- `sub_80D32C4`: blink for 10 ticks and go. (Nothing sets this action.)
local function fade(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.collision:set_region(nil)
        me.timer = 10
    end
    me.visible = bit32.btest(me.timer, 2)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D317C`: resolve the last hits; an attack ends it, a body starts
-- the stinging; then act (not while dimmed) and register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        finish(me)
        return
    end
    local hit = c.hit_flags
    if hit ~= 0 then
        if bit32.btest(hit, ENDS_IT) then
            finish(me)
            return
        end
        if bit32.btest(hit, BODIES) then
            c:set_region(nil)
            if me.action ~= STING then
                me.action = STING
                me.phase = 0
                me.phase_init = 0
            end
        end
    end
    if not battle.dimmed() then
        local a = me.action
        if a == FLY then
            fly(me)
        elseif a == STING then
            sting(me)
        elseif a == FADE then
            fade(me)
        else
            error(string.format("honey bee action %#x reads past its table", a))
        end
    end
    c:present()
end

-- `sub_80D30F4`: on its panel, flying ahead; or, without a collision
-- slot, a puff and gone (false).
local function init(me: Object): boolean
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        battle.effect(me.pos + Vec3.new(0, 0, PUFF_Z), effects.puff)
        me:free()
        return false
    end
    me:setup_collision(HIT.is, HIT.hits, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision:set_hit_effect(sparks.wood)
    sprite.palette = s.palette
    me.collision:present()
    me.vel = Vec3.new(me.facing * s.speed, me.vel.y, me.vel.z)
    -- Ticks a panel takes.
    me.timer2 = int.tdiv(PANEL_WIDTH, s.speed)
    me.lifecycle = "update"
    return true
end

bee.kind = define.kind {
    id = "rskyhny/bee",
    pool = "attack",
    state = { speed = "i32", turn_speed = "i32", target = "object", palette = "u8", turns = "u8" },
    -- `sub_80D30D0`.
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

-- `sub_80D32FE`: `what` on (x, y) for `owner` with the damage word
-- `damage`. It runs while dimmed (but stands still). Its position is the
-- spawner's registers (the panel's row, the element and 16 pixels up)
-- until its init places it; only the height lasts.
function bee.spawn(owner: Object, x: number, y: number, element: number, what: HoneyBee, damage: number): Object?
    local o = battle.spawn(bee.kind, Vec3.new(y, element, 0x10_0000))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
        local s = o.state :: State
        s.palette = what.palette
        s.speed = what.speed
        s.turn_speed = what.turn_speed
    end
    return o
end

return bee
