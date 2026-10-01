--!strict
-- An absorbed obstacle (the original's effect object #0x87, `sub_80E97F0`):
-- what an obstacle the absorbing special pulled in (DustCross's B+Back,
-- action 0x58) leaves behind. It flies in 9 ticks to a point 20 pixels in
-- front of the absorbing side's navi and, if that navi is still absorbing,
-- joins its absorbed list (up to 8). It puffs away if the battle ends or
-- it leaves the field. The obstacle framework spawns it (`sub_800F90E`,
-- `obstacle.fly_to_absorber`) with the obstacle's look (`absorbed_obstacle
-- .look { ... }`, each obstacle's own: the original's obstacle kinds), the
-- side that absorbs it, and the obstacle's animation, palette and hidden
-- sprite parts. See docs/engine/field-objects.md §4.4.

local absorbed_obstacle = {}

export type State = {
    -- How it looks (the original's obstacle kind, Param1), the side whose
    -- navi absorbs it (Param2), the obstacle's animation and palette
    -- (Param3, Param4) and hidden sprite parts (ExtraVars+8).
    look: AbsorbedLook,
    side: number,
    anim: number,
    palette: number,
    hidden_parts: number,
}

-- The absorbing special's action: while the navi is in it, the obstacle
-- can join it. (Its number, until MegaMan's weapons are definitions: the
-- absorbing special is weapon routine 0x2A's v1 module.)
local ABSORBING = 0x58

-- Ticks to reach the navi, and how far in front of it, in pixels.
local FLIGHT, AHEAD = 9, 20

-- Its height when it arrives, 16.16.
local ARRIVAL_Z = 0x10_0000

-- How many a navi keeps.
local MAX_ABSORBED = 8

-- The puff when it goes early (effect 0x12), 16 pixels up.
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local PUFF_LIFT = 0x10_0000

-- An absorbed obstacle's look, an "absorbed-look" record: what an obstacle
-- passes `obstacle.fly_to_absorber` (one of the original's obstacle kinds,
-- the sprites of `byte_80E98C0`).
function absorbed_obstacle.look(look: AbsorbedLook): AbsorbedLook
    return define.record("absorbed-look", look)
end

-- `sub_80E9810`: the obstacle's look, and the flight toward its side's navi.
local function init(me: Object)
    local s = me.state :: State
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(s.look.sprite)
    sprite.shadow = "hidden"
    local anim = s.anim
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite:step()
    sprite.palette = s.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if s.look.keeps_parts then
        sprite.hidden_parts = s.hidden_parts
    end
    -- sub_80103BC: the side's first actor, if it is a player.
    local navi = battle.player(s.side)
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
            navi:push_absorbed((me.state :: State).look, me.anim)
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

absorbed_obstacle.kind = define.kind {
    id = "absorbed-obstacle",
    pool = "effect",
    state = {
        look = "record:absorbed-look",
        side = "u8",
        anim = "u8",
        palette = "u8",
        hidden_parts = "u32",
    },
    -- `sub_80E97F0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

return absorbed_obstacle
