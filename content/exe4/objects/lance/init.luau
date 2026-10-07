--!strict
-- EXE4's lance (attack object #0x4F: init 0x080D73F8, tick 0x080D7482, spawn 0x080D7522), Lance's and DrkLance's (the
-- chips' action 0x20, its variant 6: three of them, 0x080D7540). Unlike EXE5's and EXE6's (@exelib/lance, which hit
-- once on their seventh tick), it is an attack the whole time it shows: it appears 64 pixels past its panel, slides
-- back 8 pixels a tick for 8 ticks, then blinks out over 20; each tick its hits resolve and spark, the first that
-- lands ends its region (it hits once), and its panel shows highlighted while its region stands. Its look: sprite
-- 0c-2B in the palette its spawn's first parameter names (DrkLance's 1), the swords' swing sound (0x8E).

local collision = require("../rules/collision")
local sparks = require("../lib/sparks")

local lance = {}

export type State = {
    -- Its palette (the spawn's first parameter).
    palette: number,
}

local SPRITE = asset.sprite("lance")
local SOUND = asset.sound("sword-swing")
-- How far past its panel it appears, in pixels, and how fast it slides back, 16.16.
local AHEAD, SPEED = 0x40, 0x8_0000
-- Its hits' modifier.
local HIT_MOD = 0x10
-- The actions.
local THRUST, BLINK = 0, 4

-- 0x080D74C8: slide back onto the panels; after 8 ticks, blink.
local function thrust(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    me.timer -= 1
    if me.timer ~= 0 then
        return
    end
    me.timer = 0x14
    me.action = BLINK
    me.phase = 0
end

-- 0x080D74F8: blink (hidden while the timer's bit 1 is clear), then go.
local function blink(me: Object)
    me.timer -= 1
    if me.timer == 0 then
        me.collision:set_region(nil)
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    if not bit32.btest(me.timer, 2) then
        me.visible = false
    end
end

-- 0x080D7482.
local function tick(me: Object)
    me.visible = true
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local a = me.action
    if a == THRUST then
        thrust(me)
    elseif a == BLINK then
        blink(me)
    else
        error(string.format("lance action %#x reads past its table", a))
    end
    c:present()
end

-- 0x080D73F8. False when there was no collision record for it (it is gone).
local function init(me: Object): boolean
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.shadow = "ground"
    sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), 1))
    me:set_coordinates_from_panel()
    local facing = me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x + facing * AHEAD * 0x1_0000, p.y, p.z)
    me.vel = Vec3.new(-facing * SPEED, me.vel.y, me.vel.z)
    me:set_panel_from_coordinates()
    me.timer = 8
    sprite.palette = (me.state :: State).palette
    if not me:create_collision() then
        me:free()
        return false
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
    return true
end

lance.kind = new.kind {
    id = "lance",
    pool = "attack",
    state = { palette = "u8" },
    -- 0x080D73C8: then its panel highlighted while its region stands.
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
        if me.collision.region then
            field.highlight(me.panel_x, me.panel_y)
        end
    end,
}

-- 0x080D7522: a lance of `owner`'s on (x, y) of `element`, in palette `palette`, with the damage word `damage`. (It
-- spawns at the registers' values, which its init replaces but for Z.)
local function spawn(owner: Object, x: number, y: number, element: number, z: number, palette: number, damage: number)
    local o = battle.spawn(lance.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance;
        (o.state :: State).palette = palette
    end
end

-- 0x080D7540: three lances in the far column (by the user's side: x 6, or 1 for side 1), rows 1 to 3, at the user's
-- height.
function lance.thrust(owner: Object, element: number, palette: number, damage: number)
    local x = 6 - owner.alliance * 5
    for y = 1, 3 do
        spawn(owner, x, y, element, owner.pos.z, palette, damage)
    end
end

return lance
