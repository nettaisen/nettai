--!strict
-- BodyGrd's shuriken, as BN5 and BN6 share it (BN6's attack object #0x5C,
-- `sub_80CFEC4`; BN5's #0x5C, 0x080D14F0, ShadowMan's): it dives from 180
-- pixels up and behind onto its panel in 16 ticks, highlighting it,
-- appearing with a puff halfway, hits it for two ticks, lies there 30
-- ticks and blinks out over 20. Each game makes its own kind
-- (`shuriken.make_kind`: its sprite, sound, puff, collision types, and
-- what more its dive's start does: BN5's marker over its panel).

local regions = require("../regions")

local shuriken = {}

export type State = {
    -- Its hit modifier (the original's first parameter).
    hit_mod: number,
}

-- A game's shuriken: its kind's key, its sprite, its landing's sound, the
-- puff as it shows, what it is (row 0x1C: a thrown slash) and hits, and
-- what more its dive's start does (BN5's 0x080D15E6).
export type Look = {
    id: string,
    sprite: SpriteAsset,
    land_sound: Sound,
    puff: Effect,
    self_type: CollisionType,
    target: CollisionType,
    started: ((me: Object) -> ())?,
}

local START, STEP = 0xB4_0000, 0xC_0000
local DIVE, SHOWS_AT = 0xF, 7
local LIE, BLINK = 0x1E, 0x14
local LANDED_ANIM = 1

-- `sub_80CFFB0`.
local function start(me: Object, look: Look)
    me.timer = DIVE
    me.action = 4
    if look.started then
        look.started(me)
    end
end

-- `sub_80CFFBC`: dive, highlighting the panel; shown with a puff halfway;
-- landed, hit.
local function dive(me: Object, look: Look)
    field.highlight(me.panel_x, me.panel_y)
    if me.timer == SHOWS_AT then
        me.anim, me.anim_loaded = 0, 0xFF
        me.visible = true
        battle.effect(me.pos, look.puff)
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
    battle.play_sound(look.land_sound)
    me.action = 8
    me.timer = LIE
    battle.play_sound(look.land_sound)
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

-- `sub_80CFF62`: resolve last tick's hits (a hit ends its region); at the
-- battle's end, gone; else its action, the sprite, present.
local function tick(me: Object, look: Look)
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
    local a = me.action
    if a == 0 then
        start(me, look)
    elseif a == 4 then
        dive(me, look)
    elseif a == 8 then
        lie(me)
    elseif a == 0xC then
        blink(me)
    else
        error(string.format("BodyGrd's shuriken: action %#x reads past off_80CFFA0", a))
    end
    me:update_sprite()
    c:present()
end

-- `sub_80CFEE4`: unless the battle is over or its panel is off the field:
-- high up behind its panel, hidden; its collision on no panel yet. On at
-- once.
local function init(me: Object, look: Look)
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(look.sprite)
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
    me:setup_collision(look.self_type, look.target, (me.state :: State).hit_mod)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me, look)
end

-- A game's shuriken kind.
function shuriken.make_kind(look: Look): Kind
    return define.kind {
        id = look.id,
        pool = "attack",
        state = { hit_mod = "u8" },
        -- `sub_80CFEC4`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80D006E`: a shuriken of `kind` onto (x, y) from `owner` with
-- `element`, `hit_mod` and the damage word, of its side and facing. (Its
-- position is the spawner's registers until its init places it.)
function shuriken.spawn(kind: Kind, owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    local o = battle.spawn(kind)
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
