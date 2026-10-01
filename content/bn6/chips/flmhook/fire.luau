--!strict
-- A flame of FlmHook's (the original's attack object #0xCA,
-- `sub_80DE7F4`): it swings along an arc of 160 pixels' radius around a
-- point 120 pixels behind its column and 48 down, starting at an angle
-- and lasting as long as its table says (`byte_80DE894`, `byte_80DE89C`,
-- by its speed, facing and swing), leaving afterimages. It hits the two
-- panels ahead of where it is, only on the tick it enters a panel; a hit
-- flashes the screen. Its swing (0 or 1) is which way it turns; a fast
-- one (the original's kind 1, which no BN6 chip throws) turns faster, has
-- another palette and shader, and runs while dimmed.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local trajectory = require("../../lib/trajectory")

local fire = {}

export type State = {
    -- Its swing (Param1), speed (Param2: kind 1) and hit spark (Param3).
    swing: number,
    fast: boolean,
    spark: Spark?,
    -- The arc's center (ExtraVars, ExtraVars+4), 16.16.
    center_x: number,
    center_y: number,
    -- Ticks since its last afterimage (ExtraVars+8).
    trail: number,
}

local SPRITE = asset.sprite("flame-hook-fire")
local SOUND = asset.sound("flame-hook-fire")
-- Its collision (`dword_80DEA38`): a breaking attack that hits navis, hit
-- modifier 3.
local HIT_MOD = 3
-- Where the arc's center is: behind and down, 16.16.
local CENTER_BACK, CENTER_DOWN = 0x78_0000, 0x30_0000
local COLUMN = 0x28_0000
-- The arc's radius (`byte_80DEA44`), 16.16 per unit of the sine table.
local RADIUS = 0xA000
-- By speed (`off_80DE88C`, kind 0 and 1): four bytes per facing (right,
-- left), the start angle (high byte) by swing and then the lifetime by
-- swing.
local ARCS = {
    slow = { 0x16, 0xF5, 0x1E, 0x1E, 0x6A, 0x8B, 0x1E, 0x1E },
    fast = { 0x16, 0xD2, 0x1E, 0x1E, 0x6A, 0x8A, 0x1E, 0x1E },
}
-- How fast its angle turns (`off_80DE9DC`), an afterimage every how many
-- ticks, the afterimages' shader, and its palette: slow, then fast.
local TURN_SLOW, TURN_FAST = 0x380, 0x280
local TRAIL_SLOW, TRAIL_FAST = 4, 6
local SHADER_SLOW, SHADER_FAST = 0x83E0, 0xA1EF
local AFTERIMAGE_TICKS = 0xE
-- A hit's screen flash: 10 ticks, going on while dimmed.
local FLASH_TICKS = 10

-- The sine table's entry (the game reads it with a signed load).
local function sine(angle: number): number
    return trajectory.sine(angle)
end

-- `sub_80DE9E4`: an afterimage where it is.
local function afterimage(me: Object, s: State)
    -- (Its sprite as Param1 and Param2; the lifetime word's high bytes
    -- clear: a shadow at its height. The fast one's palette, `sub_80E3428`.)
    battle.afterimage(me, me.pos, {
        sprite = SPRITE,
        anim = me.anim,
        flip = bit32.bxor(me.alliance, me.flip),
        lifetime = AFTERIMAGE_TICKS,
        color_shader = if s.fast then SHADER_FAST else SHADER_SLOW,
        palette = if s.fast then 1 else 0,
    })
end

-- `sub_80DE984`: onto the arc at its angle (the timer's high byte), turn
-- the angle, and count its life down; what is left.
local function swing(me: Object, s: State): number
    local angle = bit32.rshift(me.timer, 8)
    local c, sn = sine(angle + 64), sine(angle)
    local p = me.pos
    me.pos = Vec3.new(int.mul32(RADIUS, c) + s.center_x, int.mul32(RADIUS, sn) + s.center_y, p.z)
    local turn = if s.fast then TURN_FAST else TURN_SLOW
    -- The game reads the swing and the kind (Param1 and Param2) as one
    -- halfword here: a fast one turns 0x200 times as fast.
    local way = (s.swing + (if s.fast then 0x100 else 0)) * 2 - 1
    me.timer = me.timer + turn * way * me.facing
    local left = me.timer2 - 1
    me.timer2 = left
    return left
end

-- `sub_80DE924`.
local function burn(me: Object, s: State)
    if s.trail == 1 then
        afterimage(me, s)
    end
    local trail = s.trail + 1
    if trail >= (if s.fast then TRAIL_FAST else TRAIL_SLOW) then
        trail = 0
    end
    s.trail = trail
    if swing(me, s) == 0 then
        me:set_lifecycle("destroy")
        me.collision:set_region(nil)
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80DE8C0`.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    c:set_region(nil)
    if c.hit_flags ~= 0 then
        -- A white flash (variant 0) that goes on while dimmed (mode 1).
        battle.palette_flash(0, FLASH_TICKS, true)
    end
    local x, y = me.panel_x, me.panel_y
    local a = me.action
    if a ~= 0 then
        error(string.format("flame action %#x reads past its table", a))
    end
    burn(me, s)
    if me.lifecycle ~= "destroy" and (me.panel_x ~= x or me.panel_y ~= y) and field.valid(me.panel_x, me.panel_y) then
        c:set_region(regions.long)
    end
    me:highlight_collision_panels()
    c:present()
end

-- `sub_80DE818`.
local function init(me: Object)
    local s = me.state :: State
    local turn = bit32.bxor(me.alliance, me.flip)
    local arc = if s.fast then ARCS.fast else ARCS.slow
    local at = turn * 4 + s.swing + 1
    if arc[at + 2] == nil then
        error(string.format("flame swing %d reads past its arc table (off_80DE88C)", s.swing))
    end
    me.timer = bit32.lshift(arc[at], 8)
    me.timer2 = arc[at + 2]
    local facing = me.facing
    s.center_x = -facing * CENTER_BACK
    s.center_y = CENTER_DOWN
    -- sub_80DE8A4
    s.center_x = s.center_x + (me.panel_x - (4 - turn)) * COLUMN
    me.visible = true
    -- sub_8011504
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = if s.swing ~= 0 then 1 else 0, 0xFF
    if not me:create_collision() then
        -- The game goes on without one, writing through a null pointer.
        error("FlmHook's flame without a collision slot (sub_80DE818 doesn't check)")
    end
    me:setup_collision(collision.breaking, collision.hits_navis, HIT_MOD)
    sprite:set_flip(turn)
    sprite.palette = if s.fast then 1 else 0
    local c = me.collision
    c:set_hit_effect(s.spark)
    c:set_region(nil)
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

fire.kind = define.kind {
    id = "flmhook/fire",
    pool = "attack",
    state = {
        swing = "u8",
        fast = "bool",
        spark = "spark",
        center_x = "i32",
        center_y = "i32",
        trail = "u32",
    },
    -- `sub_80DE7F4`.
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

-- `sub_80DE960` and `sub_801155A`: a flame of `owner`'s (the hook) on
-- (x, y), of swing `swing` and speed `fast`, its hits with spark `spark`
-- and the damage word `damage`; a fast one runs while dimmed. (It spawns at
-- the registers' values, which its update replaces.)
function fire.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    swing_: number,
    fast: boolean,
    spark: Spark?,
    damage: number
): Object?
    local o = battle.spawn(fire.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        local s = o.state :: State
        s.swing, s.fast, s.spark = swing_, fast, spark
        if fast then
            o.run_while_dimmed = true
        end
    end
    return o
end

return fire
