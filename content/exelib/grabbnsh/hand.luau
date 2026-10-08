--!strict
-- GrabBnsh's and GrabRvng's hand, as EXE5 and EXE6 share it (EXE6's attack
-- object #0x46, `sub_80CCC48`; EXE5's #0x46;
-- docs/engine/dimming-chip-effects.md §3): each game makes its own kind
-- (`hand.make_kind`: its sprite, sounds and collision types). It drops from
-- 192 pixels up onto an enemy navi's panel in 24 ticks, hits it, and goes
-- once its grab animation is over. Only a round's first hand makes sounds.
-- EXE4's (#0x46, 0x080D5FF8) drops from 256 pixels and hits with hit
-- modifier 0x0B: its look's `height` and `hit_mod`.

local regions = require("../regions")

local hand = {}

export type State = {
    -- The first hand of its strike (the original's first parameter, the
    -- hands before it, is 0): it makes the sounds.
    first: boolean,
}

-- A game's hand: its kind's key, its sprite, the sounds of its fall and
-- its landing, and its hit's collision types (`dword_80CCD20`: an attack
-- that pierces invisibility, EXE6's row 0x19, hitting navis); the height it
-- drops from (16.16; none: 192 pixels) and its hit's modifier (none: 0x4B).
export type Look = {
    id: string,
    sprite: SpriteAsset,
    fall_sound: Sound,
    land_sound: Sound,
    target: CollisionType,
    self_type: CollisionType,
    height: number?,
    hit_mod: number?,
}

local HEIGHT, FALL_SPEED = 0xC0_0000, 0x8_0000
-- Its hit's modifier (`dword_80CCD20`: 0x4B, no spark, element 0).
local HIT_MOD = 0x4B
local GRAB_ANIM = 1

-- `sub_80CCCE0`: fall 8 pixels a tick; on landing, hit the panel.
local function fall(me: Object, look: Look)
    local p = me.pos
    local z = int.i32(p.z - me.vel.z)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    me.pos = Vec3.new(p.x, p.y, 0)
    if (me.state :: State).first then
        battle.play_sound(look.land_sound)
    end
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = look.target,
        self_type = look.self_type,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = look.hit_mod or HIT_MOD,
    })
    -- (sub_80C53A6 lets it resolve while dimmed; the hand's own attempt
    -- at that writes to the BIOS.)
    if h then
        h.run_while_dimmed = true
    end
    me.action = 4
end

-- `sub_80CCD24`.
local function grab(me: Object)
    me.anim = GRAB_ANIM
    me.action = 8
end

-- `sub_80CCD30`: gone once the animation is over (freed next update).
local function done(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.lifecycle = "destroy"
    end
end

-- `sub_80CCCBC`.
local function tick(me: Object, look: Look)
    local a = me.action
    if a == 0 then
        fall(me, look)
    elseif a == 4 then
        grab(me)
    elseif a == 8 then
        done(me)
    else
        error(string.format("GrabBnsh's hand: action %#x reads past off_80CCCD4", a))
    end
    me:update_sprite()
end

-- `sub_80CCC68`: high over its panel's center.
local function init(me: Object, look: Look)
    me.visible = true
    me.sprite:load(look.sprite)
    -- sprite_noShadow: the first part is drawn with the sprite.
    me.sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    me.vel = Vec3.new(me.vel.x, me.vel.y, FALL_SPEED)
    local x, y = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(x, y, look.height or HEIGHT)
    if (me.state :: State).first then
        battle.play_sound(look.fall_sound)
    end
    me.lifecycle = "update"
    tick(me, look)
end

-- A game's hand kind.
function hand.make_kind(look: Look): Kind
    return new.kind {
        id = look.id,
        pool = "attack",
        state = { first = "bool" },
        -- `sub_80CCC48`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:free()
            end
        end,
    }
end

-- `sub_80CCD4A`: a hand (of `kind`, a game's) on (x, y) with `element` and
-- the damage word, of `owner`'s side (not its facing), running while
-- dimmed; `first`: no hand came before it this strike. (Its position is
-- the spawner's registers until its init places it.)
function hand.spawn(kind: Kind, owner: Object, x: number, y: number, element: number, damage: number, first: boolean): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    (o.state :: State).first = first
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance = owner.alliance
    o.run_while_dimmed = true
    return o
end

return hand
