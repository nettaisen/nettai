--!strict
-- G-OTOSHI's frog (the DS's attack object 0xA2, 0x020A1FE4; sprite 10-46, no shadow): it falls from 192 pixels onto its
-- panel, an Aqua thrown attack hitting navis (hit modifier 1) from 20 pixels up. A hit ends it and turns its panel to sea;
-- landing on a solid panel turns it to sea too, and 20 ticks later it goes in a puff of smoke; on a hole it goes in the
-- puff at once.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")

local frog = {}

local FALL_FROM, GRAVITY, LOW = 0xC00000, -0x4000, 0x140000
local HIT_MOD = 1
local FROG = asset.sprite("frog")
local SOUND_DROP = asset.sound("drop")
-- Landed: 20 ticks; the puff 10 pixels up.
local LANDED_TICKS, PUFF_Z = 0x14, 0xA0000

-- Gone: the puff, its region off, the destroy state.
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + PUFF_Z)), effects.puff)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x020A2088: its region on from 20 pixels up; it falls; below the ground, on a hole gone, else landed.
local function fall(me: Object)
    local p, v = me.pos, me.vel
    if p.z <= LOW then
        me.collision:set_region(regions.single)
    end
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(p.x, p.y, z)
    if z >= 0 then
        return
    end
    if not field.solid(me.panel_x, me.panel_y) then
        puff(me)
        return
    end
    me:set_action(4)
end

-- 0x020A20E0: landed: its panel sea; 20 ticks, then gone.
local function landed(me: Object)
    if me.phase_init == 0 then
        me.timer = LANDED_TICKS
        field.set_type(me.panel_x, me.panel_y, "sea")
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        puff(me)
    end
end

-- 0x020A2048: its hits resolved and its spark; a hit ends it, its panel sea; else its action; its collision presented.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags == 0 then
        local a = me.action
        if a == 0 then
            fall(me)
        elseif a == 4 then
            landed(me)
        else
            error(string.format("G-OTOSHI's frog: action %#x reads past its table (0x02135C58)", a))
        end
    else
        me.visible = false
        c:set_region(nil)
        field.set_type(me.panel_x, me.panel_y, "sea")
        me:set_lifecycle("destroy")
    end
    c:present()
end

-- 0x020A1FF8: on its panel 192 pixels up; the frog (`sub_8011504`: 10-46, no shadow, animation 0), its collision (a
-- thrown attack hitting navis, hit modifier 1, no region yet); the drop's sound.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, FALL_FROM)
    local v = me.vel
    me.vel = Vec3.new(v.x, v.y, 0)
    local sprite = me.sprite
    sprite:load(FROG)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        -- (The DS goes on through the missing record.)
        error("G-OTOSHI's frog: no collision record was free")
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision:set_region(nil)
    me.collision:present()
    battle.play_sound(SOUND_DROP)
    me:set_lifecycle("update")
    tick(me)
end

frog.kind = new.kind {
    id = "g-otoshi/frog",
    pool = "attack",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x020A2120: a frog on (x, y) for `toadman`, Aqua, of his side and facing, with his damage word, running while dimmed.
function frog.spawn(toadman: Object, x: number, y: number): Object?
    local o = battle.spawn(frog.kind)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 2
        o:set_damage_word(bit32.bor(toadman.damage, bit32.lshift(toadman.stamina, 16)))
        o.alliance, o.flip = toadman.alliance, toadman.flip
        o.related1 = toadman
        o.run_while_dimmed = true
    end
    return o
end

return frog
