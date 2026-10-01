--!strict
-- An absorbed obstacle (effect object #0x87, `sub_80E97F0`): what an
-- obstacle the absorbing special pulled in (DustCross's B+Back, action
-- 0x58) leaves behind. It flies in 9 ticks to a point 20 pixels in front
-- of the absorbing side's navi and, if that navi is still absorbing, joins
-- its absorbed list (up to 8). It puffs away if the battle ends or it
-- leaves the field. The obstacle framework spawns it (`sub_800F90E`). See
-- docs/engine/field-objects.md §4.4.

export type State = {
    -- The obstacle's hidden sprite parts (ExtraVars+8), which kind 2 keeps.
    hidden_parts: number,
}

local absorbed_obstacle = {
    state = { hidden_parts = "u32" },
}

-- The absorbing special's action: while the navi is in it, the obstacle
-- can join it.
local ABSORBING = 0x58

-- Ticks to reach the navi, and how far in front of it, in pixels.
local FLIGHT, AHEAD = 9, 20

-- Its height when it arrives, 16.16.
local ARRIVAL_Z = 0x10_0000

-- The obstacle kind that keeps the obstacle's hidden parts.
local KEEPS_PARTS = 2

-- How many a navi keeps.
local MAX_ABSORBED = 8

-- The puff when it goes early.
local PUFF, PUFF_LIFT = 0x12, 0x10_0000

-- `sub_80E996E`, as `sub_800F90E` calls it for `obstacle` (of absorbed
-- kind `kind`, pulled in by side `side`): from the obstacle's position,
-- with its animation, palette, side, facing and hidden sprite parts. It
-- runs neither while paused nor while dimmed.
function absorbed_obstacle.spawn(obstacle: Object, kind: number, side: number): Object?
    local sprite = obstacle.sprite
    local o = battle.spawn_kind("absorbed-obstacle", obstacle.pos, { kind, side, obstacle.anim, sprite.palette })
    if o then
        (o.state :: State).hidden_parts = sprite.hidden_parts
        o.alliance, o.flip = obstacle.alliance, obstacle.flip
        o.run_while_paused = false
        o.run_while_dimmed = false
    end
    return o
end

-- `sub_80E9810`: the obstacle's look, and the flight toward its side's navi.
local function init(me: Object)
    me:set_panel_from_coordinates()
    local kind = me:param(1)
    local sprite_id: SpriteId? = data.objects.absorbed_sprites[kind]
    if sprite_id == nil then
        error(string.format("absorbed obstacle kind %d reads past sub_80E9810's sprite table", kind))
    end
    local sprite = me.sprite
    sprite:load(sprite_id)
    sprite.shadow = "hidden"
    local anim = me:param(3)
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite:step()
    sprite.palette = me:param(4)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if kind == KEEPS_PARTS then
        sprite.hidden_parts = (me.state :: State).hidden_parts
    end
    -- sub_80103BC: the side's first actor, if it is a player.
    local navi = battle.player(me:param(2))
    if navi == nil then
        error("no player navi to fly to: sub_80103BC returns NULL and sub_80E9810 reads its position from address 0")
    end
    me.related1 = navi
    local target = navi.pos
    local tx = target.x + int.shl(navi.facing * AHEAD, 16)
    local p = me.pos
    me.timer = FLIGHT
    me.vel = Vec3.new(
        int.tdiv(int.i32(tx - p.x), FLIGHT),
        int.tdiv(int.i32(target.y - p.y), FLIGHT),
        int.tdiv(int.i32(ARRIVAL_Z - p.z), FLIGHT)
    )
    me.visible = true
    me:set_lifecycle("update")
end

-- `sub_80E991C`: fly; once there (with its navi still absorbing), join the
-- navi's list, or keep flying if it is full.
local function fly(me: Object)
    me.pos = me.pos + me.vel
    me:set_panel_from_coordinates()
    local navi = me.related1 :: Object
    if navi.action == ABSORBING then
        local left = me.timer - 1
        me.timer = left
        if left <= 0 and #navi:absorbed() < MAX_ABSORBED then
            navi:push_absorbed(me:param(1), me.anim)
            me:set_lifecycle("destroy")
        end
    end
    me.sprite:step()
end

-- `sub_80E98DE`: gone in a puff when the battle is over or it is off the
-- field.
local function tick(me: Object)
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + PUFF_LIFT), PUFF)
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("absorbed obstacle action %#x reads past sub_80E98DE's table", a))
    end
    fly(me)
end

-- `sub_80E97F0`.
function absorbed_obstacle.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return absorbed_obstacle
