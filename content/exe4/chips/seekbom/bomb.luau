--!strict
-- EXE4's seeking bomb (attack object 0x8D, 0x080DED60: EXE4's own), which the SeekBoms throw (the throw's variant 9,
-- its thrower 0x080DEEF2). The thrower aims it at the panel of the other side's navi ahead of the user nearest in
-- columns (0x080DEF1A; between equally near ones it weighs the row against the nearest row distance so far, then the
-- lower row), else past the far edge of the field in the middle row. From the throw's release point it flies there
-- in 40 ticks along an arc falling 0x3000 a tick (aimed by `sub_8001330`, @exelib/trajectory), a thrown object
-- hitting navis (modifier 3, no spark) with no region in flight. Where it lands: on a solid panel its region (the
-- panel) for a tick, then it bursts (../../objects/sparkles: two explosions 16 ticks apart, 32 ticks of them); on a
-- hole it goes in a puff (effect row 0x15). A hit of its bursts it too; the battle over, it goes in a puff. Its
-- sprite (10-24) is turned to no side, its shadow on the ground.

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparkles = require("../../objects/sparkles")
local trajectory = require("../../lib/trajectory")

local bomb = {}

local SPRITE = asset.sprite("seek-bomb")
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

-- Its flight: 40 ticks, falling 0x3000 a tick (0x080DEFE8); its first X velocity (the init's: 3 pixels a tick toward
-- the other side, which the aim replaces).
local FLIGHT_TICKS, GRAVITY, FIRST_SPEED = 40, -0x3000, 0x3_0000
local HIT_MOD = 3
-- Its bursts: the sparkles' 32 ticks (their table's row 2, 0x080E3368) of explosions (effect row 0).
local BURST_TICKS = 0x20
-- The other side's navi on a panel, by the user's side (0x080DEFE0).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
-- With no target: past the far edge of the field (column 7, or 0 for side 1), the middle row.
local NO_TARGET_Y = 2

local FLYING, BURSTING = 0x0, 0x4

-- Its target (Param3 and Param4: the thrower's).
type State = { target_x: number, target_y: number }

-- Gone where it is in a puff (0x080DEE12, 0x080DEE7A).
local function puff(me: Object)
    battle.effect(me.pos, PUFF)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080DEE20, 0x080DEEC4: it bursts.
local function burst(me: Object)
    sparkles.spawn(me, BURST_TICKS, EXPLOSION, false)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080DEE3C): aimed on its first tick; on along its arc; landed, its region on a solid panel (and then
-- its burst), or gone on a hole.
local function fly(me: Object)
    if me.phase_init == 0 then
        local s = me.state :: State
        local x, y = field.center(s.target_x, s.target_y)
        me.vel = trajectory.in_ticks(me.pos, x, y, 0, FLIGHT_TICKS, GRAVITY)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    if not field.solid(me.panel_x, me.panel_y) then
        battle.effect(me.pos, PUFF)
        me:set_lifecycle("destroy")
        return
    end
    me.collision:set_region(regions.single)
    me.action = BURSTING
    me.phase, me.phase_init = 0, 0
end

-- 0x080DEDE8: the hits against it resolve and spark; the battle over, gone in a puff; hit, it bursts; else its
-- action, then present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        puff(me)
        return
    end
    if c.hit_flags ~= 0 then
        burst(me)
        return
    end
    local a = me.action
    if a == FLYING then
        fly(me)
    elseif a == BURSTING then
        burst(me)
    else
        error(string.format("a seeking bomb's action %#x reads past 0x080DEE34", a))
    end
    c:present()
end

-- 0x080DED84: its sprite (its shadow on the ground) in animation 1; its first X velocity; on the panel under it; a
-- thrown object hitting navis, no spark, no region; then its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 1, 0xFF
    local v = me.vel
    me.vel = Vec3.new(int.i32(me.facing * FIRST_SPEED), v.y, v.z)
    me:set_panel_from_coordinates()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(nil)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

bomb.kind = new.kind {
    id = "seekbom/bomb",
    pool = "attack",
    state = { target_x = "u8", target_y = "u8" },
    -- 0x080DED60: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080DEF1A: the panel of the other side's navi ahead of `user` nearest in columns; between equally near ones, the
-- one whose row is below the nearest row distance so far (the game compares the row itself with it), then the lower
-- row; none (0, 0) when no navi of the other side is ahead.
local function target(user: Object): (number, number)
    local ahead = {}
    for _, p in panels.filtered_except(user, ENEMY_BODY[user.alliance], 0) do
        local x = panels.x(p)
        if (user.alliance == 0 and x > user.panel_x) or (user.alliance ~= 0 and user.panel_x > x) then
            table.insert(ahead, p)
        end
    end
    local best_x, best_y, best_dx, best_dy = 0, 0, 0xFF, 0xFF
    for _, p in ahead do
        local x, y = panels.x(p), panels.y(p)
        local dx, dy = math.abs(x - user.panel_x), math.abs(y - user.panel_y)
        local take = dx < best_dx or (dx == best_dx and (y < best_dy or (y == best_dy and y < best_y)))
        if take then
            best_x, best_y, best_dx, best_dy = x, y, dx, dy
        end
    end
    return best_x, best_y
end

-- 0x080DEEF2, the throw's variant 9: a seeking bomb at the release point aimed at the user's target (else past the
-- far edge, 0x080DEEFE), of the user's side, null-element, with the damage word (0x080DEED6).
function bomb.thrower(): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local x, y = target(user)
            if x == 0 then
                x, y = 7 - 7 * user.alliance, NO_TARGET_Y
            end
            local o = battle.spawn(bomb.kind, pos)
            if o then
                o.alliance = user.alliance
                o.element = 0
                o:set_damage_word(damage)
                local s = o.state :: State
                s.target_x, s.target_y = x, y
            end
            return o
        end,
    }
end

return bomb
