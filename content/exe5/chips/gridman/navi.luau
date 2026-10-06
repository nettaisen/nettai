--!strict
-- GridMan, the navi of the GridMan chips (EXE5's actor object #0x1F, 0x080BDE5C; EXE5's own), brought by their navi
-- chip controller. He appears (and goes at once off a solid panel), stands, and dashes forward across the field,
-- zigzagging between the top and bottom rows, his grid (./grid) waiting at the far column; at each column's center
-- he snaps to its row and checks the panel (a panel that isn't solid and free sends him away), and from the far
-- column he strikes the whole enemy area (explosions over it, the camera shaking more for the columns he crossed).
-- Then he goes, and the controller moves on.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local regions = require("@exelib/regions")
local navi_chips = require("@exelib/navi_chips/navi")
local grid = require("./grid")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x60).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
    -- The column he strikes from (+5) and the columns he has crossed (+6).
    end_x: number,
    crossed: number,
    -- His grid there (+0x64).
    grid: Object?,
}

local gridman = {}

local SPRITE = asset.sprite("gridman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_DASH = asset.sound("grid-dash")
local SOUND_STRIKE = asset.sound("hit-bomb-1")
-- The explosions over the enemy area (effect 0) and the puffs behind his dash (effect 2).
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local PUFF = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 }
local HIT_MOD = 3

-- By the level (0x080BDF0C): the dark palette for DS's 4.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 0, 5 }
-- His dash: 4 pixels a tick forward, 2.5 up or down.
local DASH_X, DASH_Y = 0x4_0000, 0x2_8000
-- What a panel he crosses must be (solid) and be free of (as words).
local SOLID = 0x10
local OCCUPIED = 0x0F80_0000

-- 0x080BDE80: on his panel, on the ground, his palette by the level; his dash's speeds (up from the top row, else
-- down), the far column (6 facing right, else 1) and his grid on its middle row.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    local s = me.state :: State
    me.sprite.palette = PALETTES[s.level] or 0
    me.vel = Vec3.new(me.facing * DASH_X, if me.panel_y == 1 then DASH_Y else -DASH_Y, me.vel.z)
    s.end_x = if bit32.bxor(me.alliance, me.flip) == 0 then 6 else 1
    local g = grid.spawn(me, s.end_x, 2)
    if g then
        s.grid = g
    end
    s.crossed = 0
    me:set_lifecycle("update")
end

-- Enter the phase with a timer (a word store clears Timer2 too), or count down; true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        me.timer2 = 0
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- He is done: the controller moves on, his grid goes, and so does he.
local function done(me: Object)
    local s = me.state :: State
    local controller = s.controller
    if controller then
        navi_chip.navi_left(controller)
    end
    local g = s.grid
    if g then
        grid.remove(g)
    end
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080BDF50): he appears; 10 ticks; then he stands, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 0xA) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0x10)
    end
end

-- Action 4 (0x080BDFA0): 40 ticks standing (animation 5).
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
    end
    if countdown(me, 0x28) then
        me:set_action(8)
    end
end

-- 0x080BE17A: a puff behind him, 16 pixels ahead of his center and 8 toward where he is heading, 8 up (a random
-- 8 to 14 when he doesn't climb).
local function puff(me: Object)
    local z = 8
    local vy = me.vel.y
    if vy == 0 then
        z = battle.rng_positive() % 7 + 8
    end
    local p = me.pos
    local x = int.i32(p.x + me.facing * 0x10_0000)
    local y = int.i32(p.y + (if vy < 0 then -0x8_0000 else 0x8_0000))
    battle.effect(Vec3.new(x, y, z * 0x1_0000), PUFF, 0, 0, 0)
end

-- Action 8 (0x080BDFC8): the dash (animation 12, its sound every 32 ticks), a puff every 4 ticks.
local function dash(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = 0x20
        me.timer2 = 0
        me.anim = 12
        return
    end
    local t = me.timer + 1
    me.timer = t
    if t >= 0x20 then
        me.timer = 0
        battle.play_sound(SOUND_DASH)
    end
    local center = field.center(me.panel_x, me.panel_y)
    local p, v = me.pos, me.vel
    local old_x = p.x
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), p.z)
    if center ~= old_x and navi_chips.between(me.pos.x, old_x, center) then
        -- (0x080BE01E adds 0 to his damage.)
        s.crossed = int.u8(s.crossed + 1)
        if not field.check(me.panel_x, me.panel_y, SOLID, OCCUPIED) then
            me:set_action(0x10)
            return
        end
        local _, cy = field.center(me.panel_x, me.panel_y)
        local q = me.pos
        me.pos = Vec3.new(q.x, cy, q.z)
        if me.panel_y ~= 2 then
            me.vel = Vec3.new(v.x, -v.y, v.z)
        end
        if me.panel_x == s.end_x then
            me:set_action(0xC)
            return
        end
    end
    me:set_panel_from_coordinates()
    local n = me.timer2 + 1
    me.timer2 = n
    if n >= 4 then
        me.timer2 = 0
        puff(me)
    end
end

-- Action 0xC (0x080BE084): the strike over the enemy area (running while dimmed), its explosions, the camera
-- shaking by the columns crossed (40 ticks), and he is off the field (panel (10, 0)); 60 ticks, then he goes.
local function strike(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        local enemy = regions.area[1 - me.alliance]
        local box = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = enemy,
            hit_effect = sparks.plain,
            target = collision.nothing,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if box then
            box.run_while_dimmed = true
        end
        battle.region_effects(0, 0, enemy, me.alliance, EXPLOSION, 0)
        battle.shake_camera(bit32.rshift(s.crossed + 1, 1), 0x28)
        battle.play_sound(SOUND_STRIKE)
        me.anim = 0
        me.panel_x, me.panel_y = 10, 0
        me:set_coordinates_from_panel()
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        done(me)
    end
end

-- Action 0x10 (0x080BE120): he goes (animation 4); 3 ticks.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 3) then
        done(me)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = dash, [0xC] = strike, [0x10] = leave }

gridman.kind = new.kind {
    id = "gridman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", end_x = "u8", crossed = "u8", grid = "object" },
    -- 0x080BDE5C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            -- 0x080BDF14: the battle's end ends him (the controller moves on).
            if battle.over() then
                local controller = (me.state :: State).controller
                if controller then
                    navi_chip.navi_left(controller)
                end
                me:set_lifecycle("destroy")
            else
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("GridMan action %#x reads past his table (0x080BDF3C)", me.action))
                end
                action(me)
            end
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BE154: GridMan for `user`, reporting to `controller` (the summon table's entry 18), of a chip of `level`.
function gridman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(gridman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- GridMan's and GridMnSP's (levels 0); GridMnDS's (4).
gridman.summon = gridman.summon_at(0)
gridman.summon_dark = gridman.summon_at(4)

return gridman
