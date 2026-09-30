--!strict
-- A flame of FlmHook's (attack object #0xCA, `sub_80DE7F4`): it swings
-- along an arc of 160 pixels' radius around a point 120 pixels behind its
-- column and 48 down, starting at an angle and lasting as long as its
-- table says (`byte_80DE894`, `byte_80DE89C`, by Param2, facing and
-- Param1), leaving afterimages. It hits the two panels ahead of where it
-- is, only on the tick it enters a panel; a hit flashes the screen. Param1
-- is its swing (0 or 1, which way it turns), Param2 its kind (1: faster,
-- another palette and shader, and it runs while dimmed), Param3 its hit
-- spark.

local flame_hook_fire = {}

export type State = {
    -- The arc's center (ExtraVars, ExtraVars+4), 16.16.
    center_x: number,
    center_y: number,
    -- Ticks since its last afterimage (ExtraVars+8).
    trail: number,
}

flame_hook_fire.state = { center_x = "i32", center_y = "i32", trail = "u32" }

local SPRITE = "0c-45"
-- Its collision (`dword_80DEA38`): types and hit modifier.
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 6, 5, 3
-- The two panels it hits.
local REGION = 2
-- Where the arc's center is: behind and down, 16.16.
local CENTER_BACK, CENTER_DOWN = 0x78_0000, 0x30_0000
local COLUMN = 0x28_0000
-- The arc's radius (`byte_80DEA44`), 16.16 per unit of the sine table.
local RADIUS = 0xA000
-- By kind (`off_80DE88C`): four bytes per facing (right, left), the start
-- angle (high byte) by swing and then the lifetime by swing.
local ARCS = {
    [0] = { 0x16, 0xF5, 0x1E, 0x1E, 0x6A, 0x8B, 0x1E, 0x1E },
    [1] = { 0x16, 0xD2, 0x1E, 0x1E, 0x6A, 0x8A, 0x1E, 0x1E },
}
-- How fast its angle turns, by kind (`off_80DE9DC`).
local TURN = { [0] = 0x380, [1] = 0x280 }
-- Afterimages: every 4 ticks (6 for kind 1), their shader by kind, and
-- how long they last.
local TRAIL = { [0] = 4, [1] = 6 }
local SHADER = { [0] = 0x83E0, [1] = 0xA1EF }
local AFTERIMAGE_TICKS = 0xE
-- A hit's screen flash: 10 ticks, going on while dimmed.
local FLASH_TICKS = 10

-- `sub_80DE960` and `sub_801155A`: a flame of `owner`'s (the hook) on
-- (x, y); kind 1 runs while dimmed. (It spawns at the registers' values,
-- which its update replaces.)
function flame_hook_fire.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number
): Object?
    local o = battle.spawn_kind("flame-hook-fire", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        if o:param(2) == 1 then
            o.run_while_dimmed = true
        end
    end
    return o
end

-- The sine table's entry (the game reads it with a signed load).
local function sine(angle: number): number
    return data.rules.sine[angle] :: number
end

-- `sub_80DE9E4`: an afterimage where it is.
local function afterimage(me: Object)
    local kind = me:param(2)
    local shader = SHADER[kind] or SHADER[1]
    -- (Its sprite, 0c-45, as Param1 and Param2; no shadow.)
    local params = { 0x0C, 0x45, me.anim, bit32.bxor(me.alliance, me.flip) }
    battle.afterimage(me, me.pos, params, shader, AFTERIMAGE_TICKS, if kind == 1 then 1 else 0)
end

-- `sub_80DE984`: onto the arc at its angle (the timer's high byte), turn
-- the angle, and count its life down; what is left.
local function swing(me: Object, s: State): number
    local angle = bit32.rshift(me.timer, 8)
    local c, sn = sine(angle + 64), sine(angle)
    local p = me.pos
    me.pos = Vec3.new(int.mul32(RADIUS, c) + s.center_x, int.mul32(RADIUS, sn) + s.center_y, p.z)
    local turn = TURN[me:param(2)]
    if turn == nil then
        error(string.format("flame kind %d reads past off_80DE9DC", me:param(2)))
    end
    -- The game reads Param1 and Param2 as one halfword here.
    local way = (me:param(1) + me:param(2) * 0x100) * 2 - 1
    me.timer = me.timer + turn * way * me.facing
    local left = me.timer2 - 1
    me.timer2 = left
    return left
end

-- `sub_80DE924`.
local function burn(me: Object, s: State)
    if s.trail == 1 then
        afterimage(me)
    end
    local trail = s.trail + 1
    if trail >= (if me:param(2) ~= 0 then TRAIL[1] else TRAIL[0]) then
        trail = 0
    end
    s.trail = trail
    if swing(me, s) == 0 then
        me:set_lifecycle("destroy")
        me.collision.region = 0
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
    c.region = 0
    if c.hit_flags ~= 0 then
        -- A white flash (variant 0) that goes on while dimmed (mode 1).
        battle.palette_flash({ 0, FLASH_TICKS, 1, 0 })
    end
    local x, y = me.panel_x, me.panel_y
    local a = me.action
    if a ~= 0 then
        error(string.format("flame action %#x reads past its table", a))
    end
    burn(me, s)
    if me.lifecycle ~= "destroy" and (me.panel_x ~= x or me.panel_y ~= y) and field.valid(me.panel_x, me.panel_y) then
        c.region = REGION
    end
    c:highlight()
    c:present()
end

-- `sub_80DE818`.
local function init(me: Object)
    local s = me.state :: State
    local turn = bit32.bxor(me.alliance, me.flip)
    local kind = me:param(2)
    local swing_ = me:param(1)
    local arc = ARCS[kind]
    local at = turn * 4 + swing_ + 1
    if arc == nil or arc[at + 2] == nil then
        error(string.format("flame kind %d, swing %d reads past its arc table (off_80DE88C)", kind, swing_))
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
    me.anim, me.anim_loaded = if swing_ ~= 0 then 1 else 0, 0xFF
    if not me:create_collision() then
        -- The game goes on without one, writing through a null pointer.
        error("FlmHook's flame without a collision slot (sub_80DE818 doesn't check)")
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    sprite:set_flip(turn)
    sprite.palette = kind
    local c = me.collision
    c.hit_effect = me:param(3)
    c.region = 0
    battle.play_sound(0x158)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80DE7F4`.
function flame_hook_fire.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return flame_hook_fire
