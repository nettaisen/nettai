--!strict
-- BodyGrd's shuriken (the original's attack object #0x5C, `sub_80CFEC4`):
-- it dives from 180 pixels up and behind onto its panel in 16 ticks,
-- highlighting it, appearing with a puff halfway, hits it for two ticks,
-- lies there 30 ticks and blinks out over 20.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")

local shuriken = {}

export type State = {
    -- Its hit modifier (the original's first parameter).
    hit_mod: number,
}

local SPRITE = asset.sprite("shuriken")
local START, STEP = 0xB4_0000, 0xC_0000
local DIVE, SHOWS_AT = 0xF, 7
local LIE, BLINK = 0x1E, 0x14
local SOUND_LAND = asset.sound("panel-crack")
local LANDED_ANIM = 1
-- What it is (row 0x1C: a thrown slash) and hits.
local IS = define.collision { id = "bodygrd/shuriken", side0 = 0x80002088, side1 = 0x40002088, row_offset = 0xE0 }
local HITS = collision.hits_navis

-- `sub_80CFFB0`.
local function start(me: Object)
    me.timer = DIVE
    me.action = 4
end

-- `sub_80CFFBC`: dive, highlighting the panel; shown with a puff halfway;
-- landed, hit.
local function dive(me: Object)
    field.highlight(me.panel_x, me.panel_y)
    if me.timer == SHOWS_AT then
        me.anim, me.anim_loaded = 0, 0xFF
        me.visible = true
        battle.effect(me.pos, effects.bubbles)
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + STEP * me.facing), p.y, int.i32(p.z - STEP))
        return
    end
    me.anim = LANDED_ANIM
    me.collision:set_region(regions.single)
    battle.play_sound(SOUND_LAND)
    me.action = 8
    me.timer = LIE
    battle.play_sound(SOUND_LAND)
end

-- `sub_80D0026`: its hit lasts two ticks; it lies there 30.
local function lie(me: Object)
    if me.timer == LIE - 2 then
        me.collision:set_region(nil)
    end
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.timer, me.timer2 = BLINK, 0
    me.action = 0xC
end

-- `sub_80D0044`: blink out.
local function blink(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me:set_lifecycle("destroy")
        return
    end
    me.visible = bit32.btest(t, 2)
end

local ACTIONS: { [number]: (Object) -> () } = { [0] = start, [4] = dive, [8] = lie, [0xC] = blink }

-- `sub_80CFF62`: resolve last tick's hits (a hit ends its region); at the
-- battle's end, gone; else its action, the sprite, present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if battle.over() then
        c:set_region(nil)
        me.lifecycle = "destroy"
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("BodyGrd's shuriken: action %#x reads past off_80CFFA0", me.action))
    end
    action(me)
    me:update_sprite()
    c:present()
end

-- `sub_80CFEE4`: unless the battle is over or its panel is off the field:
-- high up behind its panel, hidden; its collision on no panel yet. On at
-- once.
local function init(me: Object)
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    local x, y = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(int.i32(x - START * me.facing), y, START)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, (me.state :: State).hit_mod)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

shuriken.kind = define.kind {
    id = "bodygrd/shuriken",
    pool = "attack",
    state = { hit_mod = "u8" },
    -- `sub_80CFEC4`.
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

-- `sub_80D006E`: a shuriken onto (x, y) from `owner` with `element`,
-- `hit_mod` and the damage word, of its side and facing. (Its position is
-- the spawner's registers until its init places it.)
function shuriken.spawn(owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    local o = battle.spawn(shuriken.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance, o.flip = owner.alliance, owner.flip
    o:set_damage_word(damage)
    ;(o.state :: State).hit_mod = hit_mod
    return o
end

return shuriken
