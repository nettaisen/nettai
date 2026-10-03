--!strict
-- DiveMan's wave (the original's attack object #0x39, `sub_80CB0DC`): a
-- wave rising on its panel (DiveMan's sprite) that washes toward the other
-- side: a hit on its panel for 32 ticks, hits on the next two panels for
-- the last 13, a splash on the second at 7 left and on the first as it
-- goes. DiveMan's own (the navi chip's) acts and animates while dimmed; the
-- navi AI's waits a dimming out. See docs/engine/chips.md §3.6.27.

local collision = require("../../rules/collision")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

local wave = {}

export type State = {
    -- DiveMan's own (Param1 4): it acts and animates while dimmed.
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("bn6:diveman")
local SOUND = asset.sound("bn6:wide-shot")
local ANIM = 0x14

-- The splash (effect 0x2B): DiveMan's sprite.
local SPLASH = define.effect { sprite = SPRITE, anim = 0x15, palette = 0 }

-- `sub_80CB208`: a hit `ahead` panels toward the other side lasting
-- `ticks` (`sub_80C53A6`: it runs while dimmed), when that panel is on
-- the field.
local function hit(me: Object, ahead: number, ticks: number)
    local x, y = me.panel_x + (1 - 2 * me.alliance) * ahead, me.panel_y
    if not field.valid(x, y) then
        return
    end
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.aqua,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
        box.timer = ticks
    end
end

-- `sub_80CB248`: the splash `ahead` panels toward the other side, when on
-- the field.
local function splash(me: Object, ahead: number)
    local x, y = me.panel_x + (1 - 2 * me.alliance) * ahead, me.panel_y
    if not field.valid(x, y) then
        return
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), SPLASH, bit32.bxor(me.alliance, me.flip))
end

-- `sub_80CB10C`: on its panel, on the ground (Z's whole part cleared: the
-- fraction is the spawner's).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

-- `sub_80CB184`: the hits and the splashes.
local function wash(me: Object)
    if me.phase == 0 then
        me.phase = 4
        hit(me, 0, 0x20)
        me.timer = 0x20
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0xD then
        hit(me, 1, 0xD)
        hit(me, 2, 0xD)
    elseif left == 7 then
        splash(me, 2)
    elseif left <= 0 then
        splash(me, 1)
        me:set_lifecycle("destroy")
        me.timer2 = 0
    end
end

-- `sub_80CB154`: gone once the battle is over; it waits a dimming out
-- unless it is DiveMan's.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if not (me.state :: State).while_dimmed and battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("DiveMan's wave action %#x reads past its table (off_80CB180)", me.action))
    end
    wash(me)
end

wave.kind = define.kind {
    id = "bn6:diveman/wave",
    pool = "attack",
    state = { while_dimmed = "bool" },
    -- `sub_80CB0DC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80CB1E2`: a wave of `owner`'s on (x, y) with `element` and the
-- damage word (flag 0x10: it runs while dimmed). `z` is the spawner's
-- register that becomes its Z, of which the init keeps the fraction.
function wave.raise(owner: Object, x: number, y: number, element: number, damage: number, while_dimmed: boolean, z: number): Object?
    local o = battle.spawn(wave.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        (o.state :: State).while_dimmed = while_dimmed
    end
    return o
end

return wave
