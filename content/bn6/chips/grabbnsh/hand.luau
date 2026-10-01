--!strict
-- GrabBnsh's and GrabRvng's hand (the original's attack object #0x46,
-- `sub_80CCC48`; docs/engine/dimming-chip-effects.md §3): it drops from
-- 192 pixels up onto an enemy navi's panel in 24 ticks, hits it, and goes
-- once its grab animation is over. Only a round's first hand makes sounds.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local hand = {}

export type State = {
    -- The first hand of its strike (the original's first parameter, the
    -- hands before it, is 0): it makes the sounds.
    first: boolean,
}

local SPRITE = asset.sprite("grab-shot")
local SOUND_FALL, SOUND_LAND = asset.sound("grab-shot-2"), asset.sound("grab-shot")
local HEIGHT, FALL_SPEED = 0xC0_0000, 0x8_0000
-- Its hit (`dword_80CCD20`): an attack that pierces invisibility (row
-- 0x19), hitting navis, hit modifier 0x4B, no spark, element 0.
local HIT_IS = collision.curse
local HIT_MOD = 0x4B
local GRAB_ANIM = 1

-- `sub_80CCCE0`: fall 8 pixels a tick; on landing, hit the panel.
local function fall(me: Object)
    local p = me.pos
    local z = int.i32(p.z - me.vel.z)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    me.pos = Vec3.new(p.x, p.y, 0)
    if (me.state :: State).first then
        battle.play_sound(SOUND_LAND)
    end
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = HIT_IS,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
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
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        fall(me)
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
local function init(me: Object)
    me.visible = true
    me.sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0xFF
    me.vel = Vec3.new(me.vel.x, me.vel.y, FALL_SPEED)
    local x, y = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(x, y, HEIGHT)
    if (me.state :: State).first then
        battle.play_sound(SOUND_FALL)
    end
    me.lifecycle = "update"
    tick(me)
end

hand.kind = define.kind {
    id = "grabbnsh/hand",
    pool = "attack",
    state = { first = "bool" },
    -- `sub_80CCC48`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- `sub_80CCD4A`: a hand on (x, y) with `element` and the damage word, of
-- `owner`'s side (not its facing), running while dimmed; `first`: no hand
-- came before it this strike. (Its position is the spawner's registers
-- until its init places it.)
function hand.spawn(owner: Object, x: number, y: number, element: number, damage: number, first: boolean): Object?
    local o = battle.spawn(hand.kind)
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
