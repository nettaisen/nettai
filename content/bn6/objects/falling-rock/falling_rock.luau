--!strict
-- A rock falling on a panel (the original's attack object #0x1D,
-- `sub_80C7D78`): it appears 104 pixels over its panel and falls, faster
-- and faster; its hit region opens once it is under 30 pixels. What it
-- hits breaks it into two chips (chip.luau); landing it breaks on a solid
-- panel, or drops into a hole with a puff. Spawned by `sub_80C7F20`: the
-- ruleset's GroundCross barrage (`sub_8012CB2`, the role
-- `kinds.falling_rock`), with the panel, element and damage word.
-- ExtraVars[0] may point at a counter of rocks in flight; the barrage
-- passes none. (The barrage's Param1, 3 to 1, nothing reads.)

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local chip = require("./chip")

local falling_rock = {}

local SPRITE = asset.sprite("falling-rock")
local SOUND = asset.sound("falling-rock")

-- Its starting height, and its fall's acceleration (16.16).
local HEIGHT, GRAVITY = 0x68_0000, 0x2000

-- It can hit once lower than this, in whole pixels.
local HIT_HEIGHT = 0x1E

-- Its collision: a thrown attack that breaks, against navis; hit modifier
-- 1; the plain spark.
local HIT_MOD = 1

-- The chips a rock breaks into: their sideways and upward speeds
-- (`dword_80C7F00`; the rock uses the first two), and their priority.
local CHIPS = { { 0x8000, 0x28000 }, { -0x8000, 0x30000 } }
local CHIP_PRIORITY = 2

-- `sub_80C7EC8`: two chips fly off from its position at ground level (in
-- front at priority 2), and a crash.
local function shatter(me: Object)
    for i = 1, 2 do
        -- The chips' Param1 is a draw's low bits plus one (unread; the
        -- draw stays).
        battle.rng()
        local p = me.pos
        chip.spawn(Vec3.new(p.x, p.y, 0), CHIPS[i][1], CHIPS[i][2], CHIP_PRIORITY)
    end
    battle.play_sound(SOUND)
end

-- `sub_80C7D98`: over its panel, falling from 104 pixels, with its hit
-- registered (region closed until low enough). Not once time is up (the
-- routine reads battle_isBattleOver's Z flag, which a KO leaves clear) or
-- when its panel is off the field: it is freed.
local function init(me: Object): boolean
    if battle.time_up() or not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return false
    end
    local x, y = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(x, y, HEIGHT)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(me.vel.x, me.vel.y, 0)
    if not me:create_collision() then
        me:free()
        return false
    end
    me:setup_collision(collision.thrown_break, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.plain)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    return true
end

-- `sub_80C7E6C`: open the region once under 30 pixels (that tick it
-- doesn't move); fall; at the ground, break or puff, and go.
local function fall(me: Object)
    if me.phase == 0 and bit32.rshift(me.pos.z, 16) < HIT_HEIGHT then
        me.collision:set_region(regions.single)
        me.phase = 4
        me.phase_init = 0
        return
    end
    local vz = me.vel.z + GRAVITY
    me.vel = Vec3.new(me.vel.x, me.vel.y, vz)
    local z = me.pos.z - vz
    me.pos = Vec3.new(me.pos.x, me.pos.y, z)
    if z > 0 then
        return
    end
    if field.solid(me.panel_x, me.panel_y) then
        shatter(me)
    else
        battle.effect(me.pos, effects.puff)
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C7E24`: resolve its hit (and the spark); a hit breaks it.
local function update(me: Object)
    me.collision:remove()
    me:hit_spark()
    if me.collision.hit_flags ~= 0 then
        shatter(me)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me.action ~= 0 then
        error("falling rock action " .. me.action .. " runs off its table (sub_80C7E24)")
    end
    fall(me)
    me:update_sprite()
    me.collision:present()
end

falling_rock.kind = define.kind {
    id = "falling-rock",
    pool = "attack",
    -- `sub_80C7D78`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if init(me) then
                update(me)
            end
        elseif l == "update" then
            update(me)
        else
            me:destroy()
        end
    end,
}

return falling_rock
