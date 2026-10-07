--!strict
-- EXE4's lightning (attack object 0x8E, 0x080DEFEC: EXE5's 0x080D8348, which no EXE5 chip uses), which LigtNin's
-- towers' controller (chips/ligtnin) strikes onto a panel holding an obstacle: with its sound (0x143) it hits the
-- eight panels around it once (region 0x10: an attack hitting navis, modifier 3, no spark), then sparks around them,
-- one every 2 ticks in its order (0x080DF0F8), each for 3 ticks (effect row 0x6B: 14-05's animation 13), and goes
-- after the eighth. Unturned, its shadow drawn with it.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")

local lightning = {}

local SPRITE = asset.sprite("lightning")
local SOUND = asset.sound("lightning")
local SPARK = new.effect { sprite = asset.sprite("hit-sparks"), anim = 13, palette = 0 }
local HIT_MOD = 3
-- The sparks' panels from the strike's (x by its facing, y), in order; one every 2 ticks, each lasting 3.
local AROUND = { { 0, -1 }, { 1, 1 }, { -1, 0 }, { 0, 1 }, { -1, -1 }, { 1, 0 }, { -1, 1 }, { 1, -1 } }
local EVERY, SPARK_TICKS = 2, 3

-- 0x080DF080: on its first tick the sparks start; a spark each time its count runs out. (At its animation's end it
-- clears no header bit: a `bics` with 0.)
local function sparks(me: Object)
    if me.phase_init == 0 then
        me.timer, me.timer2 = 0, 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t > 0 then
        return
    end
    me.timer = EVERY
    local d = AROUND[me.timer2 + 1]
    local x, y = field.center(me.panel_x + d[1] * me.facing, me.panel_y + d[2])
    local o = battle.effect(Vec3.new(x, y, 0), SPARK)
    if o then
        o.timer = SPARK_TICKS
    end
    me.timer2 += 1
    if me.timer2 >= #AROUND then
        me:set_lifecycle("destroy")
    end
end

-- 0x080DF068: its hits resolve, its region gone, the sparks, present, the sprite.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    c:set_region(nil)
    sparks(me)
    c:present()
    me:update_sprite()
end

-- 0x080DF00C: its sprite on its panel; its hit on the eight around, presented; its sound.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_region(regions.ring)
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

lightning.kind = new.kind {
    id = "lightning",
    pool = "attack",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080DF108: a lightning on (x, y) with the element and damage word, of `owner`'s side. (Its position is the
-- registers until its init places it.)
function lightning.spawn(owner: Object, x: number, y: number, element: number, damage: number)
    local o = battle.spawn(lightning.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
    end
end

return lightning
