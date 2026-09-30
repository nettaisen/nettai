--!strict
-- A fire blast (attack object #0x21, `sub_80C8388`): a flame (BlastMan's
-- sprite) that crosses the field in a straight line from where it spawns,
-- its body a hit region, highlighting the panel it is over, gone when it
-- hits something or its time is up. Param1 picks its speed (4: BlastMan's,
-- which acts and steps its sprite while dimmed; others wait a dimming
-- out), Param2 its direction: 0 left, 1 right, 2 up, 3 down. See
-- docs/engine/chips.md §3.6.14.

local blast_fire = {}

local SPRITE = "08-0C"

-- Its speed by Param1 (`byte_80C8454`), 16.16.
local SPEEDS = { [0] = 0x8_0000, 0xA_0000, 0xA_0000, 0xC_0000, 0x6_0000 }

-- By Param2: its direction (`byte_80C846C`), how far it goes (16.16,
-- `byte_80C8488`), its animation (`dword_80C8480`) and flip
-- (`dword_80C8478`).
local DIRECTIONS = { [0] = { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } }
local DISTANCES = { [0] = 0x118_0000, 0x118_0000, 0x78_0000, 0x78_0000 }
local ANIMS = { [0] = 0x11, 0x11, 0x12, 0x12 }
local FLIPS = { [0] = 1, 0, 2, 0 }

-- `sub_80C8514`: a flame on (x, y) from `owner` with `element`, the damage
-- word and `params` {speed, direction}. (Its position is the spawner's
-- registers, all of which its init overwrites.)
function blast_fire.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("blast-fire", Vec3.zero, params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80C83B8`: on its panel, on the ground, moving; collision on.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local speed, dir = SPEEDS[me:param(1)], DIRECTIONS[me:param(2)]
    if not speed or not dir then
        error(string.format("fire blast %d/%d reads past its tables (byte_80C8454, byte_80C846C)", me:param(1), me:param(2)))
    end
    me.vel = Vec3.new(int.mul32(dir[1], speed), int.mul32(dir[2], speed), me.vel.z)
    -- A BIOS division, truncating.
    me.timer = int.tdiv(DISTANCES[me:param(2)], speed)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = ANIMS[me:param(2)]
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(FLIPS[me:param(2)])
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(4, 5, 3)
    me.collision.hit_effect = 1
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C84E0`: move, highlighting the panel it is over; gone when its
-- time is up.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    field.highlight(me.panel_x, me.panel_y)
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C8498`: resolve the hits; a hit (or the battle's end) ends it;
-- act (outside a dimming unless it is BlastMan's), register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or c.hit_flags ~= 0 then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if me:param(1) == 4 or not battle.dimmed() then
        if me.action ~= 0 then
            error(string.format("fire blast action %#x reads past its table (off_80C84DC)", me.action))
        end
        fly(me)
    end
    c:present()
end

-- `sub_80C8388`.
function blast_fire.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 4 then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return blast_fire
